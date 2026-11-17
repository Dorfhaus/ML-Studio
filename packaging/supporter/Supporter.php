<?php
declare(strict_types=1);
// SPDX-License-Identifier: GPL-3.0-or-later
// Separate ML Studio authority. Never use VoicePilot's signing key or customer database.
namespace MLStudioSupporter;

const PRODUCT = 'ml-studio-supporter';
const ITEM = '833f0d3495';
const PRIVATE_DIR = '/home/users/michaellane-music/files/ml-studio-supporter';
const DELIVERY_PAGE = 'https://michaellanemusic.de/ml-studio-supporter/';
const MAIL_SENDER = 'licenses@michaellanemusic.de';
const LINK_LIFETIME = 7 * 24 * 60 * 60;

function b64(string $s): string { return rtrim(strtr(base64_encode($s), '+/', '-_'), '='); }

function sendActivationMail(string $email, string $link): bool {
    $body = "Thanks for supporting ML Studio!\n\n"
        . "Download your permanent activation here:\n$link\n\n"
        . "In ML Studio, choose Donate > Activate supporter and select the downloaded file. "
        . "This removes the Donate button and the export reminders. Keep a copy of the file for reinstalling.\n\n"
        . "This private link is valid for seven days. You can request a fresh one at " . DELIVERY_PAGE . "\n";
    return @mail($email, 'Your ML Studio supporter activation', wordwrap($body, 75), [
        'From' => 'Michael Lane Music <' . MAIL_SENDER . '>',
        'Reply-To' => 'dorfhaus2@gmail.com',
        'Content-Type' => 'text/plain; charset=UTF-8',
    ], '-f' . MAIL_SENDER);
}

final class Store {
    private \PDO $db;
    private string $seed;
    public function __construct(string $dir) {
        if (!is_dir($dir) && !mkdir($dir, 0700, true) && !is_dir($dir)) throw new \RuntimeException('Storage unavailable');
        $lock = fopen($dir.'/authority.lock', 'c');
        if (!$lock || !flock($lock, LOCK_EX)) throw new \RuntimeException('Storage busy');
        try {
            $key = $dir.'/authority.seed';
            if (!is_file($key)) {
                // Never silently replace a lost authority for an existing customer database.
                if (is_file($dir.'/supporters.sqlite')) throw new \RuntimeException('Authority missing');
                if (file_put_contents($key, random_bytes(32)) !== 32) throw new \RuntimeException('Cannot save authority');
                chmod($key, 0600);
            }
            $this->seed = (string)file_get_contents($key);
            if (strlen($this->seed) !== 32) throw new \RuntimeException('Invalid authority');
        } finally { flock($lock, LOCK_UN); fclose($lock); }
        $this->db = new \PDO('sqlite:'.$dir.'/supporters.sqlite', null, null, [\PDO::ATTR_ERRMODE=>\PDO::ERRMODE_EXCEPTION]);
        $this->db->exec('PRAGMA busy_timeout=5000');
        $this->db->exec('CREATE TABLE IF NOT EXISTS supporters (transaction_hash TEXT PRIMARY KEY, email_hash TEXT NOT NULL, certificate TEXT NOT NULL, created INTEGER NOT NULL)');
        $this->db->exec('CREATE INDEX IF NOT EXISTS supporters_email ON supporters (email_hash, created)');
        $this->db->exec('CREATE TABLE IF NOT EXISTS activation_mail (transaction_hash TEXT PRIMARY KEY, lease INTEGER NOT NULL, accepted INTEGER NOT NULL DEFAULT 0)');
        $this->db->exec('CREATE TABLE IF NOT EXISTS attempts (bucket TEXT PRIMARY KEY, count INTEGER NOT NULL, expires INTEGER NOT NULL)');
    }
    public function publicKey(): string { return bin2hex(sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($this->seed))); }
    private function digest(string $s): string { return hash_hmac('sha256', $s, $this->seed); }
    public function issue(array $e, string $verification, ?callable $send = null): array {
        if ($verification === '' || !is_string($e['verification_token'] ?? null) || !hash_equals($verification, $e['verification_token'])) throw new \DomainException('Unauthorized', 403);
        if (($e['type'] ?? '') !== 'Shop Order') return ['ignored'=>true];
        $items = $e['shop_items'] ?? [];
        $matches = array_filter(is_array($items) ? $items : [], fn($i)=>is_array($i) && ($i['direct_link_code'] ?? '') === ITEM);
        if (!$matches) return ['ignored'=>true];
        // Ko-fi gives the total order amount, not per-item paid prices. A mixed basket
        // cannot prove this entitlement reached its minimum and requires owner review.
        if (count($items) !== 1 || (int)(array_values($matches)[0]['quantity'] ?? 0) !== 1 || !empty($e['is_subscription_payment'])) throw new \DomainException('Please contact support for this order', 422);
        $amount = $e['amount'] ?? '';
        if (!is_string($amount) || !preg_match('/^([0-9]{1,7})(?:\.([0-9]{1,2}))?$/D', $amount, $m)) throw new \DomainException('Invalid amount', 422);
        $cents = (int)$m[1]*100 + (int)str_pad($m[2] ?? '', 2, '0');
        if (($e['currency'] ?? '') !== 'EUR' || $cents < 1000) throw new \DomainException('Minimum contribution is EUR 10', 422);
        $tx = self::transaction($e['kofi_transaction_id'] ?? '');
        $email = self::email($e['email'] ?? '');
        $payload = json_encode(['product'=>PRODUCT,'version'=>1,'license'=>bin2hex(random_bytes(16)), 'minimum_eur_cents'=>1000,'lifetime'=>true], JSON_THROW_ON_ERROR|JSON_UNESCAPED_SLASHES);
        $signature = sodium_crypto_sign_detached($payload, sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair($this->seed)));
        $certificate = json_encode(['payload'=>b64($payload), 'signature'=>b64($signature)], JSON_THROW_ON_ERROR);
        $transactionHash = $this->digest('tx:'.$tx);
        $emailHash = $this->digest('email:'.$email);
        $q=$this->db->prepare('INSERT OR IGNORE INTO supporters VALUES (?,?,?,?)');
        $q->execute([$transactionHash,$emailHash,$certificate,time()]);
        if ($send !== null) {
            // A replay must use the original checkout address, even if its payload changed.
            $q = $this->db->prepare('SELECT email_hash FROM supporters WHERE transaction_hash=?');
            $q->execute([$transactionHash]);
            if (!hash_equals((string)$q->fetchColumn(), $emailHash)) {
                throw new \DomainException('Order email does not match', 409);
            }
            $this->deliverOrder($email, $transactionHash, $send);
        }
        return ['received'=>true];
    }

    private function deliverOrder(string $email, string $transactionHash, callable $send): void {
        $now = time();
        $reserve = $this->db->prepare(
            'INSERT INTO activation_mail (transaction_hash,lease) VALUES (?,?)
             ON CONFLICT(transaction_hash) DO UPDATE SET lease=excluded.lease
             WHERE activation_mail.accepted=0 AND activation_mail.lease<=?'
        );
        $reserve->execute([$transactionHash, $now + 120, $now]);
        if ($reserve->rowCount() === 0) {
            $q = $this->db->prepare('SELECT accepted FROM activation_mail WHERE transaction_hash=?');
            $q->execute([$transactionHash]);
            if ((int)$q->fetchColumn() > 0) return;
            throw new \RuntimeException('Activation email is being processed');
        }

        $accepted = false;
        try {
            $accepted = $send($email, $this->downloadLink($transactionHash));
        } finally {
            $q = $this->db->prepare('UPDATE activation_mail SET lease=0,accepted=? WHERE transaction_hash=? AND lease=?');
            $q->execute([$accepted ? time() : 0, $transactionHash, $now + 120]);
        }
        // Ko-fi retries a failed callback. The saved entitlement is also available through recovery.
        if (!$accepted) throw new \RuntimeException('Activation email was not accepted');
    }

    public function emailRecovery(string $email, callable $send): void {
        $email = self::email($email);
        $emailHash = $this->digest('email:'.$email);
        $q = $this->db->prepare('SELECT transaction_hash FROM supporters WHERE email_hash=? ORDER BY created DESC LIMIT 1');
        $q->execute([$emailHash]);
        $transactionHash = $q->fetchColumn();
        if (!is_string($transactionHash)) return;

        // Limit email requests regardless of how many addresses the browser uses.
        try {
            $this->consumeLimit('recovery:'.$emailHash, 2);
        } catch (\DomainException $e) {
            if ($e->getCode() !== 429) throw $e;
            return;
        }
        if (!$send($email, $this->downloadLink($transactionHash))) {
            error_log('ML Studio supporter recovery email was not accepted');
        }
    }

    private function downloadLink(string $transactionHash): string {
        $body = $transactionHash . '.' . (time() + LINK_LIFETIME);
        // The fragment stays in the browser and out of normal HTTP request logs.
        return DELIVERY_PAGE . '#activation=' . $body . '.' . $this->digest('activation-link:' . $body);
    }

    public function claimLink(string $token): ?string {
        if (!preg_match('/^([a-f0-9]{64})\.([0-9]{1,12})\.([a-f0-9]{64})$/D', $token, $part)) return null;
        if ((int)$part[2] < time() || !hash_equals($this->digest('activation-link:' . $part[1] . '.' . $part[2]), $part[3])) {
            return null;
        }
        $q = $this->db->prepare('SELECT certificate FROM supporters WHERE transaction_hash=?');
        $q->execute([$part[1]]);
        $certificate = $q->fetchColumn();
        return is_string($certificate) ? $certificate : null;
    }

    public function claim(string $tx, string $email): ?string {
        $q=$this->db->prepare('SELECT certificate FROM supporters WHERE transaction_hash=? AND email_hash=?');
        $q->execute([$this->digest('tx:'.self::transaction($tx)), $this->digest('email:'.self::email($email))]);
        $result=$q->fetchColumn(); return is_string($result) ? $result : null;
    }
    public function rateLimit(string $ip): void {
        $this->consumeLimit('ip:'.$ip, 20);
    }

    private function consumeLimit(string $scope, int $limit): void {
        // Bounded short-lived hashes, no raw IP addresses or request logs.
        $now=time(); $bucket=$this->digest($scope.':'.intdiv($now,3600));
        $this->db->prepare('DELETE FROM attempts WHERE expires < ?')->execute([$now]);
        $q=$this->db->prepare('INSERT INTO attempts VALUES (?,1,?) ON CONFLICT(bucket) DO UPDATE SET count=count+1');
        $q->execute([$bucket,$now+3600]);
        $q=$this->db->prepare('SELECT count FROM attempts WHERE bucket=?');$q->execute([$bucket]);
        if ((int)$q->fetchColumn()>$limit) throw new \DomainException('Too many attempts. Please try again in an hour.',429);
    }
    private static function email(mixed $value): string {
        if (!is_string($value) || strlen($value)>254 || !filter_var(trim($value), FILTER_VALIDATE_EMAIL)) throw new \DomainException('Enter a valid checkout email address.',400);
        return strtolower(trim($value));
    }
    private static function transaction(mixed $value): string {
        if (!is_string($value) || !preg_match('/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/Di',trim($value))) throw new \DomainException('Check your email and Ko-fi transaction ID.',400);
        return strtolower(trim($value));
    }
}

/** Called from the existing, unchanged Ko-fi endpoint before VoicePilot dispatch.
 * Non-ML orders fall through, preserving VoicePilot's fulfilment path.
 */
function routeWebhook(): void {
    if (($_GET['action'] ?? '') !== 'kofi-webhook' || ($_SERVER['REQUEST_METHOD'] ?? '') !== 'POST') return;
    if ((int)($_SERVER['CONTENT_LENGTH'] ?? 0)>65536) { http_response_code(413); exit; }
    $event=json_decode((string)($_POST['data'] ?? ''),true);
    if (!is_array($event)) return;
    $items=$event['shop_items'] ?? [];
    if (!is_array($items) || !array_filter($items,fn($i)=>is_array($i)&&($i['direct_link_code']??'')===ITEM)) return;
    header('Content-Type: application/json; charset=utf-8');
    try {
        // Only reuse the notification token, never its licence authority.
        $config=json_decode((string)file_get_contents('/home/users/michaellane-music/files/voicepilot/config.json'),true,512,JSON_THROW_ON_ERROR);
        umask(0077);
        $result=(new Store(PRIVATE_DIR))->issue($event,(string)($config['kofi_verification_token']??''), __NAMESPACE__.'\\sendActivationMail');
        echo json_encode($result,JSON_THROW_ON_ERROR);
    } catch (\DomainException $e) { http_response_code($e->getCode()?:400); echo json_encode(['error'=>$e->getMessage()]); }
      catch (\Throwable $e) { http_response_code(503); echo '{"error":"Supporter activation temporarily unavailable. Please contact support with your receipt."}'; }
    exit;
}

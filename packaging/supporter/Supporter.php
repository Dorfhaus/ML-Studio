<?php
declare(strict_types=1);
// SPDX-License-Identifier: GPL-3.0-or-later
// Separate ML Studio authority. Never use VoicePilot's signing key or customer database.
namespace MLStudioSupporter;

const PRODUCT = 'ml-studio-supporter';
const ITEM = '833f0d3495';
const PRIVATE_DIR = '/home/users/michaellane-music/files/ml-studio-supporter';

function b64(string $s): string { return rtrim(strtr(base64_encode($s), '+/', '-_'), '='); }

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
        $this->db->exec('CREATE TABLE IF NOT EXISTS attempts (bucket TEXT PRIMARY KEY, count INTEGER NOT NULL, expires INTEGER NOT NULL)');
    }
    public function publicKey(): string { return bin2hex(sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($this->seed))); }
    private function digest(string $s): string { return hash_hmac('sha256', $s, $this->seed); }
    public function issue(array $e, string $verification): array {
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
        $q=$this->db->prepare('INSERT OR IGNORE INTO supporters VALUES (?,?,?,?)');
        $q->execute([$this->digest('tx:'.$tx),$this->digest('email:'.$email),$certificate,time()]);
        return ['received'=>true];
    }
    public function claim(string $tx, string $email): ?string {
        $q=$this->db->prepare('SELECT certificate FROM supporters WHERE transaction_hash=? AND email_hash=?');
        $q->execute([$this->digest('tx:'.self::transaction($tx)), $this->digest('email:'.self::email($email))]);
        $result=$q->fetchColumn(); return is_string($result) ? $result : null;
    }
    public function rateLimit(string $ip): void {
        // Bounded short-lived hashes, no raw IP addresses or request logs.
        $now=time(); $bucket=$this->digest('ip:'.$ip.':'.intdiv($now,3600));
        $this->db->prepare('DELETE FROM attempts WHERE expires < ?')->execute([$now]);
        $q=$this->db->prepare('INSERT INTO attempts VALUES (?,1,?) ON CONFLICT(bucket) DO UPDATE SET count=count+1');
        $q->execute([$bucket,$now+3600]);
        $q=$this->db->prepare('SELECT count FROM attempts WHERE bucket=?');$q->execute([$bucket]);
        if ((int)$q->fetchColumn()>20) throw new \DomainException('Too many attempts. Please try again in an hour.',429);
    }
    private static function email(mixed $value): string {
        if (!is_string($value) || strlen($value)>254 || !filter_var(trim($value), FILTER_VALIDATE_EMAIL)) throw new \DomainException('Check your email and Ko-fi transaction ID.',400);
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
        $result=(new Store(PRIVATE_DIR))->issue($event,(string)($config['kofi_verification_token']??''));
        echo json_encode($result,JSON_THROW_ON_ERROR);
    } catch (\DomainException $e) { http_response_code($e->getCode()?:400); echo json_encode(['error'=>$e->getMessage()]); }
      catch (\Throwable $e) { http_response_code(503); echo '{"error":"Supporter activation temporarily unavailable. Please contact support with your receipt."}'; }
    exit;
}

<?php
declare(strict_types=1);
umask(0077);
header('Cache-Control: no-store, private');
header('X-Robots-Tag: noindex, nofollow, noarchive');
header('Referrer-Policy: no-referrer');
header('X-Content-Type-Options: nosniff');
$scriptNonce = base64_encode(random_bytes(18));
header("Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-$scriptNonce'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'");
require __DIR__.'/Supporter.php';
$error='';
$notice='';
try {
    $store=new \MLStudioSupporter\Store(\MLStudioSupporter\PRIVATE_DIR);
    if (($_GET['action']??'')==='status') {
        header('Content-Type: application/json');
        echo json_encode(['ready'=>true,'product'=>\MLStudioSupporter\PRODUCT,'public_key'=>$store->publicKey(),'minimum_eur_cents'=>1000]); exit;
    }
    if ($_SERVER['REQUEST_METHOD']==='POST') {
        if ((int)($_SERVER['CONTENT_LENGTH']??0)>2048) throw new \DomainException('Request too large',413);
        $store->rateLimit($_SERVER['REMOTE_ADDR']??'unknown');
        $license = null;
        if (isset($_POST['activation'])) {
            $license = $store->claimLink((string)$_POST['activation']);
            if ($license === null) throw new \DomainException('This download link is invalid or has expired. Request a fresh link below.', 404);
        } elseif (isset($_POST['transaction'])) {
            $license = $store->claim((string)$_POST['transaction'], (string)($_POST['email']??''));
            if ($license === null) throw new \DomainException('No verified payment matched those details. Try requesting an email link below.', 404);
        } else {
            $store->emailRecovery((string)($_POST['email']??''), 'MLStudioSupporter\\sendActivationMail');
            $notice = 'If a verified purchase matches that address, a download link will arrive by email. Please check Spam or Junk and allow a few minutes for new payments to be confirmed.';
        }
        if ($license !== null) {
            header('Content-Type: application/octet-stream');
            header('Content-Disposition: attachment; filename="ML Studio Supporter.mlsupport"');
            echo $license;exit;
        }
    }
} catch (\DomainException $e) { http_response_code($e->getCode()?:400); $error=$e->getMessage(); }
  catch (\Throwable $e) { http_response_code(503); $error='Activation is temporarily unavailable. Your receipt remains valid. Please try later or contact dorfhaus2@gmail.com.'; }
?>
<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>ML Studio supporter activation</title>
<style>body{margin:0;background:#171b20;color:#eef1f4;font:17px/1.55 system-ui,sans-serif}main{max-width:660px;margin:7vh auto;padding:30px}h1{font-size:32px;line-height:1.2}label{display:block;margin-top:20px}input{box-sizing:border-box;width:100%;padding:13px;border:1px solid #667078;border-radius:7px;background:#262c32;color:white;font:inherit}button,.donate{display:inline-block;margin-top:24px;padding:13px 22px;background:#e9e554;color:#171b20;border:0;border-radius:7px;font:600 17px system-ui;text-decoration:none;cursor:pointer}a{color:#a9d9ff}.error{padding:16px;background:#50272a;border-radius:7px}.muted{color:#b8c2cc;font-size:14px}footer{border-top:1px solid #39434b;margin-top:35px;padding-top:16px}</style>
<main><p class="muted">MICHAEL LANE MUSIC · ML STUDIO</p><h1>Thank you for supporting ML Studio.</h1><p>ML Studio will always be free. A one-time contribution of <strong>€10 or more</strong> removes the Donate button and reminders after exports. Extra support is greatly appreciated.</p>
<section id="download-activation" hidden>
<h2>Your activation is ready to download</h2>
<form method="post" action="/ml-studio-supporter/">
<input type="hidden" name="activation" id="activation-token"><button type="submit">Download supporter activation</button>
</form>
</section>

<p><a class="donate" href="https://ko-fi.com/s/833f0d3495">Support ML Studio — €10 or more</a></p>
<p class="muted">Please purchase this item on its own. Once Ko-fi confirms the payment, we email a private activation link to your checkout address.</p>
<?php if($error): ?><p class="error" role="alert"><?=htmlspecialchars($error,ENT_QUOTES,'UTF-8')?></p><?php endif; ?>
<?php if($notice): ?><p role="status"><?=htmlspecialchars($notice,ENT_QUOTES,'UTF-8')?></p><?php endif; ?>
<h2>Already paid? Get a fresh link</h2><p>Enter the email address you used at Ko-fi checkout. You only need access to that inbox.</p>
<form method="post" action="/ml-studio-supporter/"><label for="email">Checkout email</label><input id="email" name="email" type="email" maxlength="254" autocomplete="email" required><button type="submit">Email my activation link</button></form>
<details><summary>Already have your Ko-fi transaction ID?</summary>
<form method="post" action="/ml-studio-supporter/"><label for="receipt-email">Checkout email</label><input id="receipt-email" name="email" type="email" maxlength="254" autocomplete="email" required><label for="transaction">Ko-fi transaction ID</label><input id="transaction" name="transaction" maxlength="36" autocomplete="off" required><button type="submit">Download with transaction ID</button></form>
</details>
<h2>Activate once. Keep it forever.</h2><ol><li>Open the link in your email and download the <strong>.mlsupport</strong> activation file.</li><li>In ML Studio, open <strong>Donate → Activate supporter</strong> or <strong>Settings → General → Activate supporter</strong> and choose that file.</li><li>Keep a backup for reinstalling or moving to another computer. Normal updates keep your activation. No ongoing internet connection or subscription is required.</li></ol>
<p>Install the <a href="https://michaellanemusic.de/ml-studio/">latest public ML Studio update</a> before activating. Audio features remain free. The activation is personal; please do not share or resell it.</p>

<footer class="muted">Need help? <a href="mailto:dorfhaus2@gmail.com">dorfhaus2@gmail.com</a><p>The activation service uses your checkout address to deliver the activation email. It stores protected hashes of your purchase email and transaction ID, along with your activation and delivery status. Payment is handled by Ko-fi and your selected payment provider. No project, audio or usage data is collected. Your statutory consumer rights remain.</p>For questions about your information, contact the support address above.</footer></main>
<script nonce="<?=$scriptNonce?>">
const activation = /^#activation=([a-f0-9]{64}\.[0-9]{1,12}\.[a-f0-9]{64})$/.exec(window.location.hash);
if (activation) {
    document.getElementById('activation-token').value = activation[1];
    document.getElementById('download-activation').hidden = false;
}
</script></html>

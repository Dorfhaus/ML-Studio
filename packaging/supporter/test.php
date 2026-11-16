<?php
declare(strict_types=1);
require __DIR__.'/Supporter.php';
use MLStudioSupporter\Store;
function check(bool $ok,string $m):void{if(!$ok)throw new RuntimeException($m);}
function rejected(callable $f,int $code):void{try{$f();}catch(DomainException $e){check($e->getCode()===$code,'wrong rejection');return;}throw new RuntimeException('not rejected');}
$dir=sys_get_temp_dir().'/ml-support-test-'.bin2hex(random_bytes(8));
try {
 $s=new Store($dir);
 $e=['verification_token'=>'test-token','type'=>'Shop Order','shop_items'=>[['direct_link_code'=>MLStudioSupporter\ITEM,'quantity'=>1]],'amount'=>'10.00','currency'=>'EUR','kofi_transaction_id'=>'12345678-1234-1234-1234-123456789abc','email'=>'Example@example.com'];
 rejected(fn()=>$s->issue($e,'wrong'),403);
 rejected(fn()=>$s->issue(array_replace($e,['amount'=>'9.99']),'test-token'),422);
 rejected(fn()=>$s->issue(array_replace($e,['currency'=>'USD']),'test-token'),422);
 rejected(fn()=>$s->issue(array_replace($e,['shop_items'=>[...$e['shop_items'],['direct_link_code'=>'other','quantity'=>1]]]),'test-token'),422);
 $s->issue($e,'test-token'); $a=$s->claim($e['kofi_transaction_id'],'example@example.com');check(is_string($a),'claim');
 $s->issue($e,'test-token');check($a===$s->claim($e['kofi_transaction_id'],$e['email']),'idempotency');
 check($s->claim($e['kofi_transaction_id'],'wrong@example.com')===null,'wrong email');
 $signed=json_decode($a,true,512,JSON_THROW_ON_ERROR);$payload=base64_decode(strtr($signed['payload'],'-_','+/'));$sig=base64_decode(strtr($signed['signature'],'-_','+/'));
 check(sodium_crypto_sign_verify_detached($sig,$payload,hex2bin($s->publicKey())),'signature');
 check(json_decode($payload,true)['lifetime']===true,'lifetime');
 $s->issue(array_replace($e,['amount'=>'25.50','kofi_transaction_id'=>'22345678-1234-1234-1234-123456789abc']),'test-token');
 check($s->claim('22345678-1234-1234-1234-123456789abc',$e['email'])!==null,'pay more');
 for($i=0;$i<20;$i++)$s->rateLimit('127.0.0.1');rejected(fn()=>$s->rateLimit('127.0.0.1'),429);
 unset($s);$s=new Store($dir);check($s->claim($e['kofi_transaction_id'],$e['email'])===$a,'reopen');
 echo json_encode(['passed'=>true,'certificate'=>$signed,'public_key'=>$s->publicKey()]);
} finally { unset($s);foreach(glob($dir.'/*')?:[] as $p)unlink($p);if(is_dir($dir))rmdir($dir); }

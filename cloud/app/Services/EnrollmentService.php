<?php
namespace App\Services;
use Illuminate\Support\Facades\{DB,Hash,Crypt};
use Illuminate\Support\Str;
final class EnrollmentService {
 public static function lookup(string $code): string { return hash_hmac('sha256',$code,(string)config('app.key')); }
 public function issue(string $schoolId, ?string $testCode=null): string {
  return DB::transaction(function() use($schoolId,$testCode) {
   if(!DB::table('schools')->where(['id'=>$schoolId,'status'=>'ACTIVE'])->lockForUpdate()->exists()) throw new \DomainException('School must be active.');
   if(!DB::table('school_card_keys')->where('school_id',$schoolId)->exists()) {
    if(DB::table('cards')->where('school_id',$schoolId)->exists()) throw new \DomainException('Existing card hashes require their original school HMAC key. Restore the key before enrollment.');
    DB::table('school_card_keys')->insert(['school_id'=>$schoolId,'encrypted_secret'=>Crypt::encryptString(bin2hex(random_bytes(32))),'created_at'=>now(),'updated_at'=>now()]);
   }
   if($testCode!==null && !app()->environment('local','testing')) throw new \DomainException('Fixed enrollment codes are test-only.');
   $code=$testCode??Str::random(40);
   DB::table('terminal_enrollment_codes')->insert(['id'=>(string)Str::uuid(),'school_id'=>$schoolId,'lookup_hash'=>self::lookup($code),'code_hash'=>Hash::make($code),'uses_remaining'=>1,'expires_at'=>now()->addHours(24),'created_at'=>now(),'updated_at'=>now()]);
   return $code;
  });
 }
}

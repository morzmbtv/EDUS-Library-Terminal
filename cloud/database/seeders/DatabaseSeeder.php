<?php
namespace Database\Seeders;

use Illuminate\Database\Seeder;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;

final class DatabaseSeeder extends Seeder
{
 public function run(): void
 {
  if(!app()->environment('local','testing')) throw new \LogicException('Demo seed is forbidden outside local/testing.');
  $now=now();
  $a='10000000-0000-4000-8000-000000000001';$b='10000000-0000-4000-8000-000000000002';
  DB::table('schools')->upsert([['id'=>$a,'name'=>'EDUS Demo School A','status'=>'ACTIVE','created_at'=>$now,'updated_at'=>$now],['id'=>$b,'name'=>'EDUS Demo School B','status'=>'ACTIVE','created_at'=>$now,'updated_at'=>$now]],['id']);
  $classA='20000000-0000-4000-8000-000000000001';$classB='20000000-0000-4000-8000-000000000002';
  DB::table('classes')->upsert([['id'=>$classA,'school_id'=>$a,'name'=>'7 A','version'=>1,'created_at'=>$now,'updated_at'=>$now],['id'=>$classB,'school_id'=>$b,'name'=>'7 B','version'=>1,'created_at'=>$now,'updated_at'=>$now]],['id']);
  $ayla='30000000-0000-4000-8000-000000000001';$dana='30000000-0000-4000-8000-000000000002';$bReader='30000000-0000-4000-8000-000000000003';
  DB::table('persons')->upsert([
   ['id'=>$ayla,'school_id'=>$a,'class_id'=>$classA,'full_name'=>'Тестовая ученица А','person_type'=>'STUDENT','class_name'=>'7 A','status'=>'ACTIVE','version'=>1,'created_at'=>$now,'updated_at'=>$now],
   ['id'=>$dana,'school_id'=>$a,'class_id'=>$classA,'full_name'=>'Тестовая ученица Б','person_type'=>'STUDENT','class_name'=>'7 A','status'=>'ACTIVE','version'=>1,'created_at'=>$now,'updated_at'=>$now],
   ['id'=>$bReader,'school_id'=>$b,'class_id'=>$classB,'full_name'=>'Тестовый ученик другой школы','person_type'=>'STUDENT','class_name'=>'7 B','status'=>'ACTIVE','version'=>1,'created_at'=>$now,'updated_at'=>$now],
  ],['id']);
  DB::table('cards')->upsert([
   ['id'=>'35000000-0000-4000-8000-000000000001','school_id'=>$a,'person_id'=>$ayla,'card_lookup_hash'=>hash_hmac('sha256','EDUSA001','test-school-a-card-key-000000000000000000000000'),'status'=>'ACTIVE','version'=>1,'created_at'=>$now,'updated_at'=>$now],
   ['id'=>'35000000-0000-4000-8000-000000000002','school_id'=>$b,'person_id'=>$bReader,'card_lookup_hash'=>hash_hmac('sha256','EDUSB001','test-school-b-card-key-000000000000000000000000'),'status'=>'ACTIVE','version'=>1,'created_at'=>$now,'updated_at'=>$now],
  ],['id']);
  $titleA='40000000-0000-4000-8000-000000000001';$titleB='40000000-0000-4000-8000-000000000002';
  DB::table('book_titles')->upsert([
   ['id'=>$titleA,'school_id'=>$a,'isbn'=>'9786012123456','title'=>'Абай жолы. 1-том','authors'=>'Мухтар Ауэзов','language'=>'Қазақша','publisher'=>'EDUS Press','publication_year'=>1984,'subject'=>'Литература','grade'=>'7','version'=>1,'created_at'=>$now,'updated_at'=>$now],
   ['id'=>$titleB,'school_id'=>$b,'isbn'=>'9786012123999','title'=>'Кітап School B','authors'=>'Demo Author','language'=>'Қазақша','publisher'=>null,'publication_year'=>null,'subject'=>null,'grade'=>null,'version'=>1,'created_at'=>$now,'updated_at'=>$now],
  ],['id']);
  DB::table('library_locations')->upsert([['id'=>'45000000-0000-4000-8000-000000000001','school_id'=>$a,'name'=>'Основной фонд','code'=>'A-MAIN','version'=>1,'created_at'=>$now,'updated_at'=>$now],['id'=>'45000000-0000-4000-8000-000000000002','school_id'=>$b,'name'=>'Фонд B','code'=>'B-MAIN','version'=>1,'created_at'=>$now,'updated_at'=>$now]],['id']);
  DB::table('book_copies')->upsert([
   ['id'=>'50000000-0000-4000-8000-000000000001','school_id'=>$a,'book_title_id'=>$titleA,'location_id'=>'45000000-0000-4000-8000-000000000001','inventory_number'=>'A-0001','barcode'=>'A-COPY-1','status'=>'AVAILABLE','version'=>1,'created_at'=>$now,'updated_at'=>$now],
   ['id'=>'50000000-0000-4000-8000-000000000002','school_id'=>$b,'book_title_id'=>$titleB,'location_id'=>'45000000-0000-4000-8000-000000000002','inventory_number'=>'B-0001','barcode'=>'B-COPY-1','status'=>'AVAILABLE','version'=>1,'created_at'=>$now,'updated_at'=>$now],
  ],['id']);
  DB::table('legacy_title_stock')->upsert([['book_title_id'=>$titleA,'school_id'=>$a,'total_quantity'=>3,'available_quantity'=>3,'version'=>1,'created_at'=>$now,'updated_at'=>$now]],['book_title_id']);
  foreach([[$a,'test-school-a-card-key-000000000000000000000000'],[$b,'test-school-b-card-key-000000000000000000000000']] as [$school,$secret]) DB::table('school_card_keys')->upsert([['school_id'=>$school,'encrypted_secret'=>\Illuminate\Support\Facades\Crypt::encryptString($secret),'created_at'=>$now,'updated_at'=>$now]],['school_id']);
  foreach([[$a,'EDUS-A1-ENROLL'],[$a,'EDUS-A2-ENROLL'],[$b,'EDUS-B1-ENROLL']] as [$school,$plain]) app(\App\Services\EnrollmentService::class)->issue($school,$plain);
 }
}

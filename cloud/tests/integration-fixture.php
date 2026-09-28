<?php
// CLI-only fixture for the real Rust/HTTP/PostgreSQL integration test.
if (PHP_SAPI !== 'cli') exit(1);
putenv('APP_ENV=testing'); $_ENV['APP_ENV']='testing'; $_SERVER['APP_ENV']='testing';
require __DIR__.'/../vendor/autoload.php';
$app=require __DIR__.'/../bootstrap/app.php';
$app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Artisan;
use App\Services\CanonicalChangeWriter;
if (!app()->environment('testing') || config('database.connections.pgsql.database') !== 'edus_library_test') throw new LogicException('Dedicated test database required.');
if (($argv[1]??'')==='reset') {
 Artisan::call('migrate:fresh',['--seed'=>true,'--force'=>true]);
 echo "Dedicated integration fixture ready.\n";
} elseif (($argv[1]??'')==='publish') {
 DB::transaction(function() {
 $school='10000000-0000-4000-8000-000000000001';
 DB::table('schools')->where('id',$school)->lockForUpdate()->first();
 $title='40000000-0000-4000-8000-000000000099';$reader='30000000-0000-4000-8000-000000000099';$now=now();
 DB::table('book_titles')->insert(['id'=>$title,'school_id'=>$school,'title'=>'Remote new edition','authors'=>'Test author','language'=>'ru','isbn'=>'9781234567897','version'=>1,'created_at'=>$now,'updated_at'=>$now]);
 DB::table('persons')->insert(['id'=>$reader,'school_id'=>$school,'full_name'=>'Тестовый новый читатель','person_type'=>'STAFF','status'=>'ACTIVE','version'=>1,'created_at'=>$now,'updated_at'=>$now]);
 $card='35000000-0000-4000-8000-000000000099';
 DB::table('cards')->insert(['id'=>$card,'school_id'=>$school,'person_id'=>$reader,'card_lookup_hash'=>hash_hmac('sha256','REMOTE99','test-school-a-card-key-000000000000000000000000'),'status'=>'ACTIVE','version'=>1,'created_at'=>$now,'updated_at'=>$now]);
 $writer=app(CanonicalChangeWriter::class);
 foreach([['BOOK_TITLE_UPSERT',$title],['PERSON_UPSERT',$reader],['CARD_UPSERT',$card]] as [$kind,$id]) $writer->upsert($school,$kind,$id,1);
 });
 echo "Canonical remote changes published.\n";
} elseif (($argv[1]??'')==='verify') {
 $school='10000000-0000-4000-8000-000000000001';
 $active=DB::table('loans')->where(['school_id'=>$school,'status'=>'ACTIVE'])->get();
 if ($active->count()!==1 || $active[0]->accounting_mode!=='LEGACY_TITLE' || (int)$active[0]->quantity-(int)$active[0]->returned_quantity!==1) throw new LogicException('Canonical PostgreSQL active loan mismatch');
 if (DB::table('terminal_operations')->where(['school_id'=>$school,'status'=>'CONFLICT'])->count()!==1) throw new LogicException('Rejected offline operation missing');
 if (DB::table('reservations')->where(['school_id'=>$school,'status'=>'WAITING'])->exists()) throw new LogicException('Cancelled reservation remains waiting');
 if (DB::table('book_copies')->where('id','50000000-0000-4000-8000-000000000001')->value('status')!=='AVAILABLE') throw new LogicException('Copy availability mismatch');
 echo "PostgreSQL canonical loans, conflict, reservation and copy assertions PASS.\n";
} else { throw new LogicException('Unknown fixture action.'); }

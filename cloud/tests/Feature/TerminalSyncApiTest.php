<?php
namespace Tests\Feature;

use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use App\Services\CanonicalChangeWriter;
use Tests\TestCase;

final class TerminalSyncApiTest extends TestCase
{
 use RefreshDatabase;
 protected bool $seed=true;
 private const SCHOOL_A='10000000-0000-4000-8000-000000000001';
 private const SCHOOL_B='10000000-0000-4000-8000-000000000002';
 private const AYLA='30000000-0000-4000-8000-000000000001';
 private const DANA='30000000-0000-4000-8000-000000000002';
 private const TITLE='40000000-0000-4000-8000-000000000001';
 private const COPY='50000000-0000-4000-8000-000000000001';
 private function enroll(string $code,string $name='Terminal A'): array { $r=$this->postJson('/api/terminal/v1/enroll',['enrollment_code'=>$code,'terminal_name'=>$name,'sync_protocol_version'=>'1','terminal_version'=>'1.1.0']);$r->assertCreated()->assertJsonPath('success',true);return $r->json('details'); }
 private function headers(string $token): array { return ['Authorization'=>'Bearer '.$token,'X-EDUS-Sync-Protocol'=>'1','X-EDUS-Terminal-Version'=>'1.1.0']; }
 private function operation(string $type,array $payload,?string $id=null,string $hash=''): array { return ['operation_id'=>$id??(string)Str::uuid(),'operation_type'=>$type,'occurred_at'=>now()->toIso8601String(),'payload_hash'=>$hash?:hash('sha256',json_encode($payload)),'payload'=>$payload]; }
 public function test_health_enrollment_and_minimal_school_scoped_bootstrap(): void {
  $this->getJson('/api/terminal/v1/health')->assertOk()->assertJsonPath('details.sync_protocol_version','1');
  $device=$this->enroll('EDUS-A1-ENROLL');$bootstrap=$this->postJson('/api/terminal/v1/bootstrap',[], $this->headers($device['device_credential']))->assertOk()->json('details');
  $this->assertSame(self::SCHOOL_A,$bootstrap['school']['id']);$this->assertCount(2,$bootstrap['persons']);$this->assertArrayNotHasKey('iin',$bootstrap['persons'][0]);$this->assertArrayNotHasKey('phone',$bootstrap['persons'][0]);$this->assertArrayNotHasKey('photo',$bootstrap['persons'][0]);
  $this->assertNotEmpty($bootstrap['next_cursor']);$this->assertTrue(collect($bootstrap['book_titles'])->every(fn($x)=>$x['school_id']===self::SCHOOL_A));
 }
 public function test_device_auth_revocation_and_school_scope_are_server_enforced(): void {
  $device=$this->enroll('EDUS-A1-ENROLL');$headers=$this->headers($device['device_credential']);
  $this->postJson('/api/terminal/v1/bootstrap',['school_id'=>self::SCHOOL_B],$headers)->assertStatus(403)->assertJsonPath('code','SCHOOL_SCOPE_VIOLATION');
  $credential=DB::table('device_credentials')->where('token_hash',hash('sha256',$device['device_credential']))->first();DB::table('device_credentials')->where('id',$credential->id)->update(['status'=>'REVOKED','revoked_at'=>now()]);
  $this->postJson('/api/terminal/v1/bootstrap',[],$headers)->assertStatus(401)->assertJsonPath('code','DEVICE_REVOKED');
 }
 public function test_issue_replay_return_partial_legacy_reservation_and_audit(): void {
  $device=$this->enroll('EDUS-A1-ENROLL');$headers=$this->headers($device['device_credential']);
  $issue=$this->operation('ISSUE',['reader_id'=>self::AYLA,'items'=>[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY']]]);$first=$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$issue]],$headers)->assertOk()->json('details.results.0');$this->assertSame('ACK',$first['status']);$loan=$first['result']['loan_ids'][0];
  $this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$issue]],$headers)->assertOk()->assertJsonPath('details.results.0.status','ACK');$this->assertSame(1,DB::table('loans')->where('book_copy_id',self::COPY)->count());
  $return=$this->operation('RETURN',['reader_id'=>self::AYLA,'items'=>[['loan_id'=>$loan,'quantity'=>1,'mode'=>'COPY']]]);$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$return]],$headers)->assertOk()->assertJsonPath('details.results.0.status','ACK');$this->assertSame('AVAILABLE',DB::table('book_copies')->where('id',self::COPY)->value('status'));$copyChange=DB::table('sync_changes')->where('entity_type','BOOK_COPY_UPSERT')->latest('sequence')->first();$this->assertSame('FULL',$copyChange->payload_kind);$this->assertSame(1,$copyChange->payload_schema_version);$copyPayload=json_decode($copyChange->payload,true,512,JSON_THROW_ON_ERROR);$this->assertSame(self::COPY,$copyPayload['id']);$this->assertSame(self::SCHOOL_A,$copyPayload['school_id']);$this->assertArrayHasKey('inventory_number',$copyPayload);$this->assertArrayHasKey('deleted_at',$copyPayload);
  $legacy=$this->operation('ISSUE',['reader_id'=>self::AYLA,'items'=>[['title_id'=>self::TITLE,'quantity'=>2,'mode'=>'LEGACY_TITLE']]]);$legacyResult=$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$legacy]],$headers)->assertOk()->json('details.results.0.result');$legacyLoan=$legacyResult['loan_ids'][0];$partial=$this->operation('RETURN',['reader_id'=>self::AYLA,'items'=>[['loan_id'=>$legacyLoan,'quantity'=>1,'mode'=>'LEGACY_TITLE']]]);$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$partial]],$headers)->assertOk();$this->assertSame(1,DB::table('loans')->where('id',$legacyLoan)->value('returned_quantity'));
  $reserve=$this->operation('RESERVE',['reader_id'=>self::DANA,'title_id'=>self::TITLE]);$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$reserve]],$headers)->assertOk()->assertJsonPath('details.results.0.status','ACK');$again=$this->operation('RESERVE',['reader_id'=>self::DANA,'title_id'=>self::TITLE]);$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$again]],$headers)->assertOk()->assertJsonPath('details.results.0.code','RESERVATION_ALREADY_EXISTS');
  $this->assertGreaterThanOrEqual(4,DB::table('audit_log')->where('school_id',self::SCHOOL_A)->count());
 }
 public function test_idempotency_mismatch_delta_cursor_and_tombstone(): void {
  $device=$this->enroll('EDUS-A1-ENROLL');$headers=$this->headers($device['device_credential']);$id=(string)Str::uuid();$one=$this->operation('RESERVE',['reader_id'=>self::DANA,'title_id'=>self::TITLE],$id);$two=$this->operation('RESERVE',['reader_id'=>self::DANA,'title_id'=>self::TITLE,'retry_marker'=>'changed'],$id);
  $this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$one]],$headers)->assertOk()->assertJsonPath('details.results.0.status','ACK');$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$two]],$headers)->assertOk()->assertJsonPath('details.results.0.code','IDEMPOTENCY_CONFLICT');
  app(CanonicalChangeWriter::class)->tombstone(self::SCHOOL_A,'PERSON_UPSERT',self::AYLA,2);$changes=$this->getJson('/api/terminal/v1/changes',$headers)->assertOk()->json('details');$this->assertTrue(collect($changes['changes'])->contains(fn($x)=>$x['entity_type']==='PERSON_UPSERT'&&$x['operation']==='DELETE'&&$x['payload_kind']==='TOMBSTONE'));$this->getJson('/api/terminal/v1/changes?cursor='.$changes['next_cursor'],$headers)->assertOk();
 }
 public function test_two_terminals_conflict_and_pull_preserves_cloud_authority(): void {
  $a=$this->enroll('EDUS-A1-ENROLL','Terminal A1');$b=$this->enroll('EDUS-A2-ENROLL','Terminal A2');$ha=$this->headers($a['device_credential']);$hb=$this->headers($b['device_credential']);$this->postJson('/api/terminal/v1/bootstrap',[],$ha)->assertOk();$this->postJson('/api/terminal/v1/bootstrap',[],$hb)->assertOk();
  $bIssue=$this->operation('ISSUE',['reader_id'=>self::AYLA,'items'=>[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY']]]);$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$bIssue]],$hb)->assertOk()->assertJsonPath('details.results.0.status','ACK');
  $aIssue=$this->operation('ISSUE',['reader_id'=>self::DANA,'items'=>[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY']]]);$this->postJson('/api/terminal/v1/operations',['sync_protocol_version'=>'1','operations'=>[$aIssue]],$ha)->assertOk()->assertJsonPath('details.results.0.code','BOOK_COPY_ALREADY_ON_LOAN');
  $this->assertSame(1,DB::table('loans')->where(['book_copy_id'=>self::COPY,'status'=>'ACTIVE'])->count());$changes=$this->getJson('/api/terminal/v1/changes',$ha)->assertOk()->json('details.changes');$this->assertTrue(collect($changes)->contains(fn($x)=>$x['entity_type']==='BOOK_COPY_UPSERT'&&$x['payload']['status']==='ON_LOAN'));
 }

 public function test_payload_hash_cannot_authorize_a_different_payload(): void {
  $d=$this->enroll('EDUS-A1-ENROLL');$h=$this->headers($d['device_credential']);
  $op=$this->operation('ISSUE',['reader_id'=>self::AYLA,'items'=>[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY']]]);
  $op['payload_json']=json_encode($op['payload']);$op['payload']['reader_id']=self::DANA;
  $this->postJson('/api/terminal/v1/operations',['operations'=>[$op]],$h)->assertOk()->assertJsonPath('details.results.0.code','VALIDATION_ERROR');
  $this->assertDatabaseCount('loans',0);$this->assertDatabaseCount('idempotency_receipts',0);
 }
 public function test_local_loan_id_survives_issue_return_and_canonical_delta(): void {
  $d=$this->enroll('EDUS-A1-ENROLL');$h=$this->headers($d['device_credential']);$loan=(string)Str::uuid();
  $issue=$this->operation('ISSUE',['reader_id'=>self::AYLA,'items'=>[['copyId'=>self::COPY,'titleId'=>self::TITLE,'loanId'=>$loan,'quantity'=>1,'mode'=>'COPY']]]);
  $this->postJson('/api/terminal/v1/operations',['operations'=>[$issue]],$h)->assertOk()->assertJsonPath('details.results.0.result.loan_ids.0',$loan);
  $return=$this->operation('ACCEPT',['reader_id'=>self::AYLA,'items'=>[['loanId'=>$loan,'quantity'=>1,'mode'=>'COPY']]]);
  $this->postJson('/api/terminal/v1/operations',['operations'=>[$return]],$h)->assertOk()->assertJsonPath('details.results.0.status','ACK');
  $this->assertDatabaseHas('loans',['id'=>$loan,'status'=>'RETURNED','returned_quantity'=>1]);
 }
 public function test_conflict_receipt_is_stable_after_stock_changes_and_type_is_bound(): void {
  $d=$this->enroll('EDUS-A1-ENROLL');$h=$this->headers($d['device_credential']);
  DB::table('book_copies')->where('id',self::COPY)->update(['status'=>'REPAIR']);
  $op=$this->operation('ISSUE',['reader_id'=>self::AYLA,'items'=>[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY']]]);
  $send=fn($o)=>$this->postJson('/api/terminal/v1/operations',['operations'=>[$o]],$h)->assertOk();
  $send($op)->assertJsonPath('details.results.0.code','BOOK_COPY_ALREADY_ON_LOAN');
  DB::table('book_copies')->where('id',self::COPY)->update(['status'=>'AVAILABLE']);
  $send($op)->assertJsonPath('details.results.0.code','BOOK_COPY_ALREADY_ON_LOAN');
  $op['operation_type']='ACCEPT';$send($op)->assertJsonPath('details.results.0.code','IDEMPOTENCY_CONFLICT');
  $this->assertDatabaseCount('loans',0);
 }
 public function test_invalid_batch_rolls_back_and_empty_issue_is_not_success(): void {
  $d=$this->enroll('EDUS-A1-ENROLL');$h=$this->headers($d['device_credential']);
  foreach ([[],[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY'],['title_id'=>self::TITLE,'quantity'=>0,'mode'=>'LEGACY_TITLE']]] as $items) {
   $op=$this->operation('ISSUE',['reader_id'=>self::AYLA,'items'=>$items]);
   $this->postJson('/api/terminal/v1/operations',['operations'=>[$op]],$h)->assertOk()->assertJsonPath('details.results.0.code','VALIDATION_ERROR');
  }
  $this->assertDatabaseCount('loans',0);$this->assertDatabaseHas('book_copies',['id'=>self::COPY,'status'=>'AVAILABLE']);
 }
 public function test_cross_school_reader_cannot_issue_local_copy(): void {
  $d=$this->enroll('EDUS-A1-ENROLL');$h=$this->headers($d['device_credential']);
  $foreign=DB::table('persons')->where('school_id',self::SCHOOL_B)->value('id');
  $op=$this->operation('ISSUE',['reader_id'=>$foreign,'items'=>[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY']]]);
  $this->postJson('/api/terminal/v1/operations',['operations'=>[$op]],$h)->assertOk()->assertJsonPath('details.results.0.code','READER_INACTIVE');
  $this->assertDatabaseCount('loans',0);
 }

 public function test_old_terminal_and_revoked_school_are_fail_closed(): void {
  $this->postJson('/api/terminal/v1/enroll',['enrollment_code'=>'EDUS-A1-ENROLL','terminal_name'=>'old','sync_protocol_version'=>'1','terminal_version'=>'1.0.0'])->assertStatus(409)->assertJsonPath('code','TERMINAL_VERSION_UNSUPPORTED');
  $d=$this->enroll('EDUS-A1-ENROLL');$h=$this->headers($d['device_credential']);$h['X-EDUS-Terminal-Version']='1.0.1';
  $this->postJson('/api/terminal/v1/bootstrap',[],$h)->assertStatus(409);
  DB::table('schools')->where('id',self::SCHOOL_A)->update(['status'=>'INACTIVE']);
  $this->postJson('/api/terminal/v1/bootstrap',[],$this->headers($d['device_credential']))->assertStatus(401);
 }
 public function test_conflict_contains_scoped_canonical_repair_and_hash_ignores_object_order(): void {
  $d=$this->enroll('EDUS-A1-ENROLL');$h=$this->headers($d['device_credential']);
  $op=$this->operation('ISSUE',['reader_id'=>self::AYLA,'items'=>[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY']]]);
  $op['payload_json']=json_encode($op['payload']);$op['payload']=array_reverse($op['payload'],true);
  $this->postJson('/api/terminal/v1/operations',['operations'=>[$op]],$h)->assertOk()->assertJsonPath('details.results.0.status','ACK');
  $conflict=$this->operation('ISSUE',['reader_id'=>self::DANA,'items'=>[['copy_id'=>self::COPY,'quantity'=>1,'mode'=>'COPY']]]);
  $response=$this->postJson('/api/terminal/v1/operations',['operations'=>[$conflict]],$h)->assertOk()->json('details.results.0');
  $this->assertSame('CONFLICT',$response['status']);$this->assertNotEmpty($response['canonical']);
  foreach($response['canonical'] as $row){$this->assertSame(self::SCHOOL_A,$row['payload']['school_id']);$this->assertSame('FULL',$row['payload_kind']);}
 }
 public function test_database_rejects_cross_school_foreign_keys(): void {
  $this->expectException(\Illuminate\Database\QueryException::class);
  DB::table('cards')->insert(['id'=>(string)Str::uuid(),'school_id'=>self::SCHOOL_B,'person_id'=>self::AYLA,'card_lookup_hash'=>str_repeat('e',64),'status'=>'ACTIVE','version'=>1,'created_at'=>now(),'updated_at'=>now()]);
 }
 public function test_bootstrap_keeps_deleted_parent_for_referential_integrity(): void {
  DB::table('persons')->where('id',self::AYLA)->update(['deleted_at'=>now(),'status'=>'INACTIVE']);
  $d=$this->enroll('EDUS-A1-ENROLL');$data=$this->postJson('/api/terminal/v1/bootstrap',[],$this->headers($d['device_credential']))->assertOk()->json('details');
  $person=collect($data['persons'])->firstWhere('id',self::AYLA);$this->assertNotNull($person['deleted_at']);
  foreach($data['cards'] as $card){$this->assertTrue(collect($data['persons'])->contains('id',$card['person_id']));}
 }
}

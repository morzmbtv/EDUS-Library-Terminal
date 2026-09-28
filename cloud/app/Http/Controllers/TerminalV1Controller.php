<?php
namespace App\Http\Controllers;

use App\Services\TerminalOperationService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;

final class TerminalV1Controller extends Controller
{
    public function __construct(private readonly TerminalOperationService $operations) {}
    private function ok(mixed $details,int $status=200): JsonResponse { return response()->json(['success'=>true,'details'=>$details],$status); }
    private function fail(string $code,string $message,int $status=422,array $details=[]): JsonResponse { return response()->json(['success'=>false,'code'=>$code,'message'=>$message]+($details?['details'=>$details]:[]),$status); }
    private function terminal(Request $r): object { return $r->attributes->get('edus.terminal'); }
    private function protocol(Request $r): ?JsonResponse { $actual=(string)$r->header('X-EDUS-Sync-Protocol',$r->input('sync_protocol_version',''));$expected=(string)config('library-cloud.sync_protocol_version');return $actual===$expected?null:$this->fail('SYNC_PROTOCOL_UNSUPPORTED','Версия протокола терминала не поддерживается.',409,['required'=>$expected,'minimum_terminal_version'=>config('library-cloud.minimum_terminal_version')]); }
    private function validated(Request $r,array $rules): array|JsonResponse { $v=validator($r->all(),$rules);return $v->fails()?$this->fail('VALIDATION_ERROR','Параметры запроса некорректны.',422,$v->errors()->toArray()):$v->validated(); }
    private function cursor(string $school,int $seq): string { $body=rtrim(strtr(base64_encode(json_encode(['school'=>$school,'sequence'=>$seq])),'+/','-_'),'=');return $body.'.'.hash_hmac('sha256',$body,(string)config('library-cloud.cursor_secret')); }
    private function decodeCursor(?string $cursor,string $school): int|JsonResponse { if(!$cursor)return 0;[$body,$sig]=array_pad(explode('.',$cursor,2),2,null);if(!$body||!$sig||!hash_equals(hash_hmac('sha256',$body,(string)config('library-cloud.cursor_secret')),$sig))return $this->fail('VALIDATION_ERROR','Курсор синхронизации недействителен.');$json=json_decode(base64_decode(strtr($body,'-_','+/'),true),true);if(!is_array($json)||($json['school']??null)!==$school||(!is_int($json['sequence']??null)||$json['sequence']<0))return $this->fail('SCHOOL_SCOPE_VIOLATION','Курсор принадлежит другой школе.',403);return $json['sequence']; }
    public function health(): JsonResponse { return $this->ok(['api_version'=>config('library-cloud.api_version'),'sync_protocol_version'=>config('library-cloud.sync_protocol_version'),'minimum_terminal_version'=>config('library-cloud.minimum_terminal_version'),'status'=>'READY']); }
    public function enroll(Request $r): JsonResponse
    {
        $data=$this->validated($r,['enrollment_code'=>'required|string|min:12|max:256','terminal_name'=>'required|string|max:120','sync_protocol_version'=>'required|string','terminal_version'=>'required|string|max:32']);
        if($data instanceof JsonResponse)return $data;
        if($data['sync_protocol_version']!==(string)config('library-cloud.sync_protocol_version'))return $this->fail('SYNC_PROTOCOL_UNSUPPORTED','Версия протокола терминала не поддерживается.',409);
        if (!preg_match('/^\d+\.\d+\.\d+$/',$data['terminal_version']) || version_compare($data['terminal_version'],config('library-cloud.minimum_terminal_version'),'<')) return $this->fail('TERMINAL_VERSION_UNSUPPORTED','Обновите терминал перед подключением.',409);
        $code=DB::table('terminal_enrollment_codes')->where('lookup_hash',\App\Services\EnrollmentService::lookup($data['enrollment_code']))->where('uses_remaining','>',0)->where(fn($q)=>$q->whereNull('expires_at')->orWhere('expires_at','>',now()))->first();
        if($code && !Hash::check($data['enrollment_code'],$code->code_hash)) $code=null;
        if(!$code)return $this->fail('DEVICE_UNAUTHORIZED','Код регистрации недействителен.',401);
        return DB::transaction(function() use ($code,$data) {
            $locked=DB::table('terminal_enrollment_codes')->where('id',$code->id)->lockForUpdate()->first();
            if(!$locked||$locked->uses_remaining<1||($locked->expires_at&&$locked->expires_at<=now())) return $this->fail('DEVICE_UNAUTHORIZED','Код регистрации уже использован или истёк.',401);
            $school=DB::table('schools')->where(['id'=>$locked->school_id,'status'=>'ACTIVE'])->lockForUpdate()->first();
            if(!$school) return $this->fail('DEVICE_UNAUTHORIZED','Школа недоступна.',401);
            $encrypted=DB::table('school_card_keys')->where('school_id',$locked->school_id)->value('encrypted_secret');
            if(!$encrypted) return $this->fail('SCHOOL_KEY_NOT_CONFIGURED','Ключ карт школы не настроен.',409);
            $cardSecret=\Illuminate\Support\Facades\Crypt::decryptString($encrypted);
            $terminalId=(string)Str::uuid();$token=Str::random(72);$now=now();
            DB::table('terminals')->insert(['id'=>$terminalId,'school_id'=>$locked->school_id,'name'=>$data['terminal_name'],'status'=>'ACTIVE','protocol_version'=>config('library-cloud.sync_protocol_version'),'created_at'=>$now,'updated_at'=>$now]);
            DB::table('device_credentials')->insert(['id'=>(string)Str::uuid(),'terminal_id'=>$terminalId,'token_hash'=>hash('sha256',$token),'status'=>'ACTIVE','created_at'=>$now,'updated_at'=>$now]);
            DB::table('terminal_enrollment_codes')->where('id',$locked->id)->decrement('uses_remaining');
            DB::table('device_sync_state')->insert(['terminal_id'=>$terminalId,'school_id'=>$locked->school_id,'last_pull_sequence'=>0,'created_at'=>$now,'updated_at'=>$now]);
            return $this->ok(['terminal_id'=>$terminalId,'school_id'=>$locked->school_id,'device_credential'=>$token,'card_hmac_secret'=>$cardSecret,'protocol'=>['api_version'=>config('library-cloud.api_version'),'sync_protocol_version'=>config('library-cloud.sync_protocol_version'),'minimum_terminal_version'=>config('library-cloud.minimum_terminal_version')]],201);
        });
    }
    public function bootstrap(Request $r): JsonResponse {
        if ($error=$this->protocol($r)) return $error;
        $t=$this->terminal($r);
        return DB::transaction(function () use ($t) {
            $school=DB::table('schools')->where('id',$t->school_id)->sharedLock()->first();
            $writer=app(\App\Services\CanonicalChangeWriter::class);
            $result=['school'=>['id'=>$school->id,'name'=>$school->name,'status'=>$school->status]];
            $sets=[
                ['classes','classes','CLASS_UPSERT'], ['persons','persons','PERSON_UPSERT'],
                ['cards','cards','CARD_UPSERT'], ['face_templates','face_templates','FACE_TEMPLATE_UPSERT'],
                ['locations','library_locations','LOCATION_UPSERT'], ['book_titles','book_titles','BOOK_TITLE_UPSERT'],
                ['book_copies','book_copies','BOOK_COPY_UPSERT'], ['legacy_title_stock','legacy_title_stock','LEGACY_STOCK_UPSERT'],
                ['active_loans','loans','LOAN_UPSERT'], ['active_reservations','reservations','RESERVATION_UPSERT'],
            ];
            foreach ($sets as [$key,$table,$kind]) {
                $query=DB::table($table)->where('school_id',$t->school_id);
                // Include minimal deleted parents required by active loans/cards; deleted_at remains authoritative.
                if ($table==='loans') $query->where('status','ACTIVE');
                if ($table==='reservations') $query->where('status','WAITING');
                $result[$key]=$query->get()->map(fn($row)=>$writer->project($kind,(array)$row))->all();
            }
            $max=(int)(DB::table('sync_changes')->where('school_id',$t->school_id)->max('sequence')??0);
            $result['next_cursor']=$this->cursor($t->school_id,$max);
            $result['library_policies']=['offline_allowed'=>true];
            return $this->ok($result);
        });
    }
    public function changes(Request $r): JsonResponse {if($error=$this->protocol($r))return $error;$t=$this->terminal($r);$from=$this->decodeCursor($r->query('cursor'),$t->school_id);if($from instanceof JsonResponse)return $from;return DB::transaction(function() use ($t,$from) {DB::table('schools')->where('id',$t->school_id)->sharedLock()->first();$changes=DB::table('sync_changes')->where('school_id',$t->school_id)->where('sequence','>',$from)->orderBy('sequence')->limit(500)->get()->map(fn($c)=>['sequence'=>$c->sequence,'entity_type'=>$c->entity_type,'entity_id'=>$c->entity_id,'version'=>$c->version,'operation'=>$c->operation,'payload_kind'=>$c->payload_kind,'payload_schema_version'=>(int)$c->payload_schema_version,'payload'=>$c->payload?json_decode($c->payload,true):null,'created_at'=>$c->created_at]);$next=$changes->isEmpty()?$from:(int)$changes->last()['sequence'];DB::table('device_sync_state')->updateOrInsert(['terminal_id'=>$t->id],['school_id'=>$t->school_id,'last_pull_sequence'=>$next,'last_sync_at'=>now(),'updated_at'=>now()]);return $this->ok(['changes'=>$changes,'next_cursor'=>$this->cursor($t->school_id,$next)]);});}
    public function operations(Request $r): JsonResponse {if($error=$this->protocol($r))return $error;$max=(int)config('library-cloud.max_batch_operations');$data=$this->validated($r,['operations'=>"required|array|min:1|max:$max",'operations.*.operation_id'=>'required|uuid','operations.*.operation_type'=>'required|string|in:ISSUE,RETURN,ACCEPT,RESERVE,issue,return,accept,reserve','operations.*.occurred_at'=>'required|date','operations.*.payload_hash'=>'required|string|regex:/^[a-f0-9]{64}$/i','operations.*.payload'=>'required','operations.*.payload_json'=>'nullable|string']);if($data instanceof JsonResponse)return $data;$t=$this->terminal($r);$results=[];foreach($data['operations'] as $op)$results[]=$this->operations->apply($t,$op);return $this->ok(['results'=>$results]);}
    public function operation(Request $r,string $operationId): JsonResponse {$t=$this->terminal($r);$result=DB::table('idempotency_receipts')->where(['school_id'=>$t->school_id,'operation_id'=>$operationId,'terminal_id'=>$t->id])->first();return $result?$this->ok(['operation_id'=>$operationId,'result'=>json_decode($result->result,true)]):$this->fail('VALIDATION_ERROR','Операция не найдена.',404);}
}


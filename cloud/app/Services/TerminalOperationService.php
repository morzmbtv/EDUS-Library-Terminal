<?php
namespace App\Services;

use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;

final class TerminalOperationService
{
    public function __construct(private readonly CanonicalChangeWriter $changes) {}

    public function apply(object $terminal, array $operation): array
    {
        $id = $operation['operation_id'];
        $type = strtoupper($operation['operation_type']);
        if ($type === 'RETURN') $type = 'ACCEPT';
        $hash = strtolower($operation['payload_hash']);
        $payloadJson = $operation['payload_json'] ?? json_encode($operation['payload'], JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        $decoded = json_decode($payloadJson, true);
        if (!is_array($decoded) || $this->canonicalPayload($decoded) !== $this->canonicalPayload($operation['payload']) || !hash_equals(hash('sha256', $payloadJson), $hash)) {
            return ['operation_id' => $id, 'status' => 'CONFLICT', 'code' => 'VALIDATION_ERROR'];
        }
        return DB::transaction(function () use ($terminal, $operation, $id, $type, $hash, $payloadJson, $decoded) {
            // Serialize operation IDs across schools without revealing another school's receipt.
            DB::select('SELECT pg_advisory_xact_lock(hashtextextended(?, 0))', ['operation:'.$id]);
            // All canonical publishers use this lock before sequence allocation. Pull readers
            // share it, so a cursor cannot jump over an allocated but uncommitted event.
            DB::table('schools')->where('id', $terminal->school_id)->lockForUpdate()->first();
            $existing = DB::table('idempotency_receipts')->where('operation_id', $id)->first();
            if ($existing) {
                if ($existing->school_id !== $terminal->school_id || $existing->terminal_id !== $terminal->id
                    || $existing->operation_type !== $type || !hash_equals($existing->payload_hash, $hash)) {
                    return ['operation_id' => $id, 'status' => 'CONFLICT', 'code' => 'IDEMPOTENCY_CONFLICT'];
                }
                $saved = json_decode($existing->result, true, 512, JSON_THROW_ON_ERROR);
                return isset($saved['status']) ? $saved : ['operation_id' => $id, 'status' => 'ACK', 'result' => $saved];
            }
            try {
                // Savepoint: failed domain changes roll back, but their definitive receipt remains.
                $result = DB::transaction(function () use ($terminal, $id, $type, $decoded) {
                    $payload = $this->normalize($type, $decoded);
                    return match ($type) {
                        'ISSUE' => $this->issue($terminal, $id, $payload),
                        'ACCEPT' => $this->accept($terminal, $id, $payload),
                        'RESERVE' => $this->reserve($terminal, $id, $payload),
                        default => throw new \DomainException('VALIDATION_ERROR'),
                    };
                });
                $response = ['operation_id' => $id, 'status' => 'ACK', 'result' => $result];
            } catch (\DomainException $error) {
                $response = ['operation_id' => $id, 'status' => 'CONFLICT', 'code' => $error->getMessage()];
            } catch (QueryException $error) {
                // Disk/network/server faults are not domain conflicts. Caller must keep its outbox.
                if (!in_array($error->getCode(), ['23505', '23514', '23503'], true)) throw $error;
                $response = ['operation_id' => $id, 'status' => 'CONFLICT', 'code' => 'VERSION_CONFLICT'];
            }
            if ($response['status'] === 'CONFLICT') $response['canonical'] = $this->conflictProjection($terminal, $type, $decoded);
            $now = now();
            DB::table('terminal_operations')->insert(['id'=>(string)Str::uuid(),'school_id'=>$terminal->school_id,'terminal_id'=>$terminal->id,'operation_id'=>$id,'operation_type'=>$type,'occurred_at'=>$operation['occurred_at'],'received_at'=>$now,'payload_hash'=>$hash,'payload'=>$payloadJson,'result'=>json_encode($response, JSON_THROW_ON_ERROR),'status'=>$response['status'],'created_at'=>$now,'updated_at'=>$now]);
            DB::table('idempotency_receipts')->insert(['id'=>(string)Str::uuid(),'school_id'=>$terminal->school_id,'terminal_id'=>$terminal->id,'operation_id'=>$id,'operation_type'=>$type,'payload_hash'=>$hash,'result'=>json_encode($response, JSON_THROW_ON_ERROR),'received_at'=>$now,'created_at'=>$now,'updated_at'=>$now]);
            if ($response['status'] === 'CONFLICT') $this->audit($terminal,$type,'REJECTED',$id,null,null,null,$response['code']);
            return $response;
        });
    }

    /** A school-scoped, transaction-consistent repair set. Never accepts school_id from input. */
    private function conflictProjection(object $terminal, string $type, array $raw): array
    {
        $payload = $this->normalize($type, $raw);
        $ids = [];
        $collect = function ($value) use (&$collect, &$ids) {
            if (!is_array($value)) return;
            foreach ($value as $key => $part) {
                if (in_array($key, ['title_id','titleId','copy_id','copyId','loan_id','loanId','reservation_id'], true) && is_string($part) && Str::isUuid($part)) $ids[] = $part;
                elseif (is_array($part)) $collect($part);
            }
        };
        $collect($payload);
        $school = $terminal->school_id;
        $titles = DB::table('book_titles')->where('school_id',$school)->whereIn('id',$ids)->pluck('id')->all();
        foreach (['book_copies','loans','reservations'] as $table) {
            $titles = array_merge($titles,DB::table($table)->where('school_id',$school)->whereIn('id',$ids)->pluck('book_title_id')->all());
        }
        $titles = array_values(array_unique($titles));
        $rows = [];
        foreach (['BOOK_TITLE_UPSERT'=>'book_titles','BOOK_COPY_UPSERT'=>'book_copies','LEGACY_STOCK_UPSERT'=>'legacy_title_stock','LOAN_UPSERT'=>'loans','RESERVATION_UPSERT'=>'reservations'] as $kind=>$table) {
            $rows[$kind] = DB::table($table)->where('school_id',$school)->whereIn($table==='book_titles'?'id':'book_title_id',$titles)->get();
        }
        $readers = $rows['LOAN_UPSERT']->pluck('reader_id')->merge($rows['RESERVATION_UPSERT']->pluck('reader_id'))->unique()->all();
        $rows['PERSON_UPSERT'] = DB::table('persons')->where('school_id',$school)->whereIn('id',$readers)->get();
        $rows['CLASS_UPSERT'] = DB::table('classes')->where('school_id',$school)->whereIn('id',$rows['PERSON_UPSERT']->pluck('class_id')->filter()->unique()->all())->get();
        $rows['LOCATION_UPSERT'] = DB::table('library_locations')->where('school_id',$school)->whereIn('id',$rows['BOOK_COPY_UPSERT']->pluck('location_id')->filter()->unique()->all())->get();
        $result=[];
        foreach ($rows as $kind=>$entities) foreach($entities as $entity) {
            $dto=$this->changes->project($kind,(array)$entity);
            $result[]=['entity_type'=>$kind,'entity_id'=>$dto['id'],'version'=>$dto['version'],'operation'=>'UPSERT','payload_kind'=>'FULL','payload_schema_version'=>1,'payload'=>$dto];
        }
        return $result;
    }

    private function canonicalPayload(mixed $value): mixed
    {
        if (!is_array($value)) return $value;
        $list=array_is_list($value);
        foreach($value as &$item) $item=$this->canonicalPayload($item);
        unset($item);
        if (!$list) ksort($value,SORT_STRING);
        return $value;
    }

    private function items(array $payload): array
    {
        $items = $payload['items'] ?? null;
        if (!is_array($items) || !array_is_list($items) || count($items) < 1 || count($items) > 200) throw new \DomainException('VALIDATION_ERROR');
        foreach ($items as $item) {
            if (!is_array($item) || !is_int($item['quantity'] ?? null) || $item['quantity'] < 1 || $item['quantity'] > 10000
                || !in_array($item['mode'] ?? null, ['COPY', 'LEGACY_TITLE'], true)
                || ($item['mode'] === 'COPY' && $item['quantity'] !== 1)) throw new \DomainException('VALIDATION_ERROR');
        }
        return $items;
    }

    private function entityId(string $operation, string $kind, int $index, mixed $supplied = null): string
    {
        if ($supplied !== null) {
            if (!is_string($supplied) || !Str::isUuid($supplied)) throw new \DomainException('VALIDATION_ERROR');
            return $supplied;
        }
        $bytes = substr(hash('sha256', "edus:v1:{$operation}:{$kind}:{$index}", true), 0, 16);
        $bytes[6] = chr((ord($bytes[6]) & 15) | 64);
        $bytes[8] = chr((ord($bytes[8]) & 63) | 128);
        $hex = bin2hex($bytes);
        return substr($hex,0,8).'-'.substr($hex,8,4).'-'.substr($hex,12,4).'-'.substr($hex,16,4).'-'.substr($hex,20);
    }
    private function normalize(string $type,mixed $payload): array {
        if(is_array($payload)&&array_is_list($payload)) return match($type) {
            'ISSUE','RETURN','ACCEPT'=>['reader_id'=>$payload[0]??null,'items'=>$payload[1]??[]],
            'RESERVE'=>['reader_id'=>$payload[0]??null,'title_id'=>$payload[1]??null], default=>[]
        };
        return is_array($payload)?$payload:[];
    }
    private function reader(object $t,string $id): object {
        $reader=DB::table('persons')->where(['id'=>$id,'school_id'=>$t->school_id,'status'=>'ACTIVE'])->whereNull('deleted_at')->first();
        if(!$reader) throw new \DomainException('READER_INACTIVE'); return $reader;
    }
    private function issue(object $t,string $operationId,array $payload): array {
        $reader=$this->reader($t,(string)($payload['reader_id']??'')); $loans=[];$copies=[];$quantity=0;
        foreach($this->items($payload) as $index => $item) {
            $mode=strtoupper((string)($item['mode']??'')); $qty=(int)($item['quantity']??0);
            if($mode==='COPY') {
                $copyId=$item['copy_id']??$item['copyId']??null;
                $copy=DB::table('book_copies')->where(['id'=>$copyId,'school_id'=>$t->school_id])->lockForUpdate()->first();
                if(!$copy || $copy->deleted_at || (isset($item['titleId']) && $item['titleId'] !== $copy->book_title_id) || (isset($item['title_id']) && $item['title_id'] !== $copy->book_title_id)) throw new \DomainException('VALIDATION_ERROR');
                if($copy->status!=='AVAILABLE') throw new \DomainException('BOOK_COPY_ALREADY_ON_LOAN');
                $loanId=$this->entityId($operationId,'loan',$index,$item['loan_id']??$item['loanId']??null); $now=now();
                DB::table('book_copies')->where('id',$copy->id)->update(['status'=>'ON_LOAN','version'=>$copy->version+1,'updated_at'=>$now]);
                DB::table('loans')->insert(['id'=>$loanId,'school_id'=>$t->school_id,'reader_id'=>$reader->id,'book_title_id'=>$copy->book_title_id,'book_copy_id'=>$copy->id,'accounting_mode'=>'COPY','quantity'=>1,'returned_quantity'=>0,'issued_at'=>$now,'status'=>'ACTIVE','created_terminal_id'=>$t->id,'version'=>1,'created_at'=>$now,'updated_at'=>$now]);
                $this->changes->upsert($t->school_id,'BOOK_COPY_UPSERT',$copy->id,$copy->version+1);
                $this->changes->upsert($t->school_id,'LOAN_UPSERT',$loanId,1);
                $copies[]=$copy->id; $loans[]=$loanId; $quantity++;
            } elseif($mode==='LEGACY_TITLE') {
                $titleId=$item['title_id']??$item['titleId']??null;
                $stock=DB::table('legacy_title_stock')->where(['book_title_id'=>$titleId,'school_id'=>$t->school_id])->lockForUpdate()->first();
                if(!$stock||!DB::table('book_titles')->where(['id'=>$titleId,'school_id'=>$t->school_id])->whereNull('deleted_at')->exists()||$qty<1||$stock->available_quantity<$qty) throw new \DomainException('BOOK_COPY_NOT_AVAILABLE');
                $loanId=$this->entityId($operationId,'loan',$index,$item['loan_id']??$item['loanId']??null);$now=now();
                DB::table('legacy_title_stock')->where('book_title_id',$titleId)->update(['available_quantity'=>$stock->available_quantity-$qty,'version'=>$stock->version+1,'updated_at'=>$now]);
                DB::table('loans')->insert(['id'=>$loanId,'school_id'=>$t->school_id,'reader_id'=>$reader->id,'book_title_id'=>$titleId,'accounting_mode'=>'LEGACY_TITLE','quantity'=>$qty,'returned_quantity'=>0,'issued_at'=>$now,'status'=>'ACTIVE','created_terminal_id'=>$t->id,'version'=>1,'created_at'=>$now,'updated_at'=>$now]);
                $this->changes->upsert($t->school_id,'LEGACY_STOCK_UPSERT',$titleId,$stock->version+1);
                $this->changes->upsert($t->school_id,'LOAN_UPSERT',$loanId,1);
                $loans[]=$loanId;$quantity+=$qty;
            } else throw new \DomainException('VALIDATION_ERROR');
        }
        $result=['operation_id'=>$operationId,'type'=>'issue','quantity'=>$quantity,'copy_ids'=>$copies,'loan_ids'=>$loans];
        $this->audit($t,'ISSUE','ACK',$operationId,$reader->id,null,$copies[0]??null); return $result;
    }
    private function accept(object $t,string $operationId,array $payload): array {
        $reader=$this->reader($t,(string)($payload['reader_id']??'')); $loans=[];$copies=[];$quantity=0;
        foreach($this->items($payload) as $index => $item) {
            $loanId=$item['loan_id']??$item['loanId']??null;
            $loan=DB::table('loans')->where(['id'=>$loanId,'school_id'=>$t->school_id,'reader_id'=>$reader->id,'status'=>'ACTIVE'])->lockForUpdate()->first();
            if(!$loan) throw new \DomainException('LOAN_NOT_ACTIVE');
            if (($item['mode']??null) !== $loan->accounting_mode || (isset($item['titleId']) && $item['titleId'] !== $loan->book_title_id) || (isset($item['copyId']) && $item['copyId'] !== $loan->book_copy_id)) throw new \DomainException('VALIDATION_ERROR');
            $qty=(int)($item['quantity']??1);$remaining=$loan->quantity-$loan->returned_quantity;
            if($qty<1||$qty>$remaining) throw new \DomainException('VALIDATION_ERROR');
            $closed=$qty===$remaining;$now=now();
            DB::table('loans')->where('id',$loan->id)->update(['returned_quantity'=>$loan->returned_quantity+$qty,'returned_at'=>$closed?$now:null,'status'=>$closed?'RETURNED':'ACTIVE','version'=>$loan->version+1,'updated_at'=>$now]);
            if($loan->accounting_mode==='COPY') {
                $copy=DB::table('book_copies')->where('id',$loan->book_copy_id)->lockForUpdate()->first();
                DB::table('book_copies')->where('id',$copy->id)->update(['status'=>'AVAILABLE','version'=>$copy->version+1,'updated_at'=>$now]);
                $this->changes->upsert($t->school_id,'BOOK_COPY_UPSERT',$copy->id,$copy->version+1);$copies[]=$copy->id;
            } else {
                $stock=DB::table('legacy_title_stock')->where(['book_title_id'=>$loan->book_title_id,'school_id'=>$t->school_id])->lockForUpdate()->first();
                DB::table('legacy_title_stock')->where('book_title_id',$loan->book_title_id)->update(['available_quantity'=>$stock->available_quantity+$qty,'version'=>$stock->version+1,'updated_at'=>$now]);
                $this->changes->upsert($t->school_id,'LEGACY_STOCK_UPSERT',$loan->book_title_id,$stock->version+1);
            }
            $this->changes->upsert($t->school_id,'LOAN_UPSERT',$loan->id,$loan->version+1);$loans[]=$loan->id;$quantity+=$qty;
        }
        $result=['operation_id'=>$operationId,'type'=>'accept','quantity'=>$quantity,'copy_ids'=>$copies,'loan_ids'=>$loans];$this->audit($t,'RETURN','ACK',$operationId,$reader->id,null,$copies[0]??null);return $result;
    }
    private function reserve(object $t,string $operationId,array $payload): array {
        if (($payload['action']??null)==='CANCEL') {
            $reader=$this->reader($t,(string)($payload['reader_id']??''));
            $row=DB::table('reservations')->where(['id'=>$payload['reservation_id']??null,'reader_id'=>$reader->id,'school_id'=>$t->school_id,'status'=>'WAITING'])->lockForUpdate()->first();
            if(!$row) throw new \DomainException('RESERVATION_NOT_ACTIVE');
            DB::table('reservations')->where('id',$row->id)->update(['status'=>'CANCELLED','version'=>$row->version+1,'updated_at'=>now()]);
            $this->changes->upsert($t->school_id,'RESERVATION_UPSERT',$row->id,$row->version+1);
            $this->audit($t,'RESERVE','ACK',$operationId,$reader->id,$row->book_title_id,null);
            return ['operation_id'=>$operationId,'type'=>'reserve','quantity'=>0,'reservation_id'=>$row->id,'copy_ids'=>[],'loan_ids'=>[]];
        }
        $reader=$this->reader($t,(string)($payload['reader_id']??''));$titleId=(string)($payload['title_id']??$payload['titleId']??'');
        if(!DB::table('book_titles')->where(['id'=>$titleId,'school_id'=>$t->school_id])->whereNull('deleted_at')->exists()) throw new \DomainException('VALIDATION_ERROR');
        if(DB::table('reservations')->where(['reader_id'=>$reader->id,'book_title_id'=>$titleId,'status'=>'WAITING'])->exists()) throw new \DomainException('RESERVATION_ALREADY_EXISTS');
        $id=$this->entityId($operationId,'reservation',0,$payload['reservation_id']??null);$now=now();DB::table('reservations')->insert(['id'=>$id,'school_id'=>$t->school_id,'reader_id'=>$reader->id,'book_title_id'=>$titleId,'status'=>'WAITING','version'=>1,'created_at'=>$now,'updated_at'=>$now]);
        $position=DB::table('reservations')->where(['school_id'=>$t->school_id,'book_title_id'=>$titleId,'status'=>'WAITING'])->count();$this->changes->upsert($t->school_id,'RESERVATION_UPSERT',$id,1);
        $result=['operation_id'=>$operationId,'type'=>'reserve','quantity'=>1,'reservation_id'=>$id,'queue_position'=>$position,'copy_ids'=>[],'loan_ids'=>[]];$this->audit($t,'RESERVE','ACK',$operationId,$reader->id,$titleId,null);return $result;
    }

    private function audit(object $t,string $action,string $result,?string $op,?string $reader,?string $title,?string $copy,?string $error=null): void { DB::table('audit_log')->insert(['id'=>(string)Str::uuid(),'school_id'=>$t->school_id,'terminal_id'=>$t->id,'operation_id'=>$op,'action'=>$action,'reader_id'=>$reader,'book_title_id'=>$title,'book_copy_id'=>$copy,'occurred_at'=>now(),'received_at'=>now(),'result'=>$result,'error_code'=>$error,'created_at'=>now(),'updated_at'=>now()]); }
}

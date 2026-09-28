<?php

namespace App\Services;

use Illuminate\Support\Facades\DB;
use JsonException;

/**
 * Writes the canonical terminal projection after a Cloud mutation.
 * The selected row is read inside the caller's transaction, so every UPSERT
 * contains the complete post-mutation state rather than an implementation patch.
 */
final class CanonicalChangeWriter
{
    public const SCHEMA_VERSION = 1;

    public function upsert(string $schoolId, string $entityType, string $entityId, int $version): void
    {
        DB::transaction(function () use ($schoolId, $entityType, $entityId, $version) {
        DB::table('schools')->where('id', $schoolId)->lockForUpdate()->first();
        $payload = $this->payload($schoolId, $entityType, $entityId);
        if ($payload === null) {
            throw new \LogicException("Cannot publish canonical {$entityType}: {$entityId} was not found in its school scope.");
        }
        if (($payload['version'] ?? null) !== $version) throw new \LogicException('Canonical version mismatch.');
        $this->insert($schoolId, $entityType, $entityId, $version, 'UPSERT', 'FULL', $payload);
        });
    }

    public function tombstone(string $schoolId, string $entityType, string $entityId, int $version, ?string $deletedAt = null): void
    {
        DB::transaction(function () use ($schoolId, $entityType, $entityId, $version, $deletedAt) {
        DB::table('schools')->where('id', $schoolId)->lockForUpdate()->first();
        $this->insert($schoolId, $entityType, $entityId, $version, 'DELETE', 'TOMBSTONE', [
            'id' => $entityId,
            'school_id' => $schoolId,
            'version' => $version,
            'deleted_at' => $deletedAt ?? now()->toIso8601String(),
        ]);
        });
    }

    /** @return array<string, mixed>|null */
    public function payload(string $schoolId, string $entityType, string $entityId): ?array
    {
        $row = match ($entityType) {
            'CLASS_UPSERT' => DB::table('classes')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            'PERSON_UPSERT' => DB::table('persons')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            'CARD_UPSERT' => DB::table('cards')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            'FACE_TEMPLATE_UPSERT' => DB::table('face_templates')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            'LOCATION_UPSERT' => DB::table('library_locations')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            'BOOK_TITLE_UPSERT' => DB::table('book_titles')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            'BOOK_COPY_UPSERT' => DB::table('book_copies')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            'LEGACY_STOCK_UPSERT' => DB::table('legacy_title_stock')->where(['book_title_id' => $entityId, 'school_id' => $schoolId])->first(),
            'LOAN_UPSERT' => DB::table('loans')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            'RESERVATION_UPSERT' => DB::table('reservations')->where(['id' => $entityId, 'school_id' => $schoolId])->first(),
            default => throw new \LogicException("Unsupported canonical entity type {$entityType}."),
        };
        if ($row === null) {
            return null;
        }
        return $this->project($entityType, (array) $row);
    }

    public function project(string $entityType, array $source): array
    {
        $fields = match ($entityType) {
            'CLASS_UPSERT' => ['id','school_id','name','version','deleted_at','updated_at'],
            'PERSON_UPSERT' => ['id','school_id','full_name','person_type','class_id','class_name','position_name','status','version','deleted_at','updated_at'],
            'CARD_UPSERT' => ['id','school_id','person_id','card_lookup_hash','status','version','updated_at'],
            'FACE_TEMPLATE_UPSERT' => ['id','school_id','person_id','template_ciphertext','model_id','model_version','template_version','version','consent_reference','consent_status','enrolled_at','revoked_at','updated_at'],
            'LOCATION_UPSERT' => ['id','school_id','name','code','version','deleted_at','updated_at'],
            'BOOK_TITLE_UPSERT' => ['id','school_id','isbn','title','authors','language','publisher','publication_year','subject','grade','version','deleted_at','updated_at'],
            'BOOK_COPY_UPSERT' => ['id','school_id','book_title_id','location_id','inventory_number','barcode','status','version','deleted_at','updated_at'],
            'LEGACY_STOCK_UPSERT' => ['book_title_id','school_id','total_quantity','available_quantity','version','updated_at'],
            'LOAN_UPSERT' => ['id','school_id','reader_id','book_title_id','book_copy_id','accounting_mode','quantity','returned_quantity','issued_at','due_at','returned_at','status','created_terminal_id','version','updated_at'],
            'RESERVATION_UPSERT' => ['id','school_id','reader_id','book_title_id','status','version','created_at','updated_at'],
        };
        $payload = [];
        foreach ($fields as $field) {
            $payload[$field] = $source[$field] ?? null;
        }
        if ($entityType === 'LEGACY_STOCK_UPSERT') {
            $payload['id'] = $payload['book_title_id'];
            $payload['deleted_at'] = null;
        }
        return $payload;
    }

    /** @param array<string, mixed> $payload */
    private function insert(string $schoolId, string $entityType, string $entityId, int $version, string $operation, string $payloadKind, array $payload): void
    {
        try {
            $encoded = json_encode($payload, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        } catch (JsonException $error) {
            throw new \LogicException("Cannot encode canonical {$entityType} payload.", previous: $error);
        }
        DB::table('sync_changes')->insert([
            'school_id' => $schoolId,
            'entity_type' => $entityType,
            'entity_id' => $entityId,
            'version' => $version,
            'operation' => $operation,
            'payload_kind' => $payloadKind,
            'payload_schema_version' => self::SCHEMA_VERSION,
            'payload' => $encoded,
            'created_at' => now(),
        ]);
    }
}
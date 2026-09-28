<?php
use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
return new class extends Migration {
    public function up(): void {
        Schema::table('idempotency_receipts', fn(Blueprint $t) => $t->string('operation_type')->default('UNKNOWN'));
        DB::statement("UPDATE idempotency_receipts r SET operation_type=CASE WHEN o.operation_type='RETURN' THEN 'ACCEPT' ELSE o.operation_type END FROM terminal_operations o WHERE r.operation_id=o.operation_id");
        Schema::table('face_templates', fn(Blueprint $t) => $t->unsignedBigInteger('version')->default(1));
        DB::statement('ALTER TABLE loans ADD CONSTRAINT loans_returned_bounds CHECK (returned_quantity >= 0 AND returned_quantity <= quantity)');
    }
    public function down(): void {
        DB::statement('ALTER TABLE loans DROP CONSTRAINT loans_returned_bounds');
        Schema::table('face_templates', fn(Blueprint $t) => $t->dropColumn('version'));
        Schema::table('idempotency_receipts', fn(Blueprint $t) => $t->dropColumn('operation_type'));
    }
};

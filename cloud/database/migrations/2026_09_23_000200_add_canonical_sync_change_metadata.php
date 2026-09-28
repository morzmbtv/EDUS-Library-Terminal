<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration {
    public function up(): void {
        Schema::table('sync_changes', function (Blueprint $table): void {
            $table->string('payload_kind', 16)->default('FULL')->after('operation');
            $table->unsignedSmallInteger('payload_schema_version')->default(1)->after('payload_kind');
        });
    }

    public function down(): void {
        Schema::table('sync_changes', function (Blueprint $table): void {
            $table->dropColumn(['payload_kind', 'payload_schema_version']);
        });
    }
};
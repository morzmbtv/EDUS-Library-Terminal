<?php
use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;
return new class extends Migration {
 public function up(): void {
  Schema::create('school_card_keys',function(Blueprint $t){$t->foreignUuid('school_id')->primary()->constrained('schools');$t->text('encrypted_secret');$t->timestampsTz();});
  Schema::table('terminal_enrollment_codes',fn(Blueprint $t)=>$t->string('lookup_hash',64)->nullable()->unique());
 }
 public function down(): void {
  Schema::table('terminal_enrollment_codes',fn(Blueprint $t)=>$t->dropColumn('lookup_hash'));
  Schema::dropIfExists('school_card_keys');
 }
};

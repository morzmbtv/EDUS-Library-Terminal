<?php
use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;
return new class extends Migration {
 private array $parents=['classes','persons','book_titles','book_copies','library_locations','terminals'];
 private array $relations=[
  ['persons','class_id','classes'],['cards','person_id','persons'],['face_templates','person_id','persons'],
  ['book_copies','book_title_id','book_titles'],['book_copies','location_id','library_locations'],
  ['legacy_title_stock','book_title_id','book_titles'],['loans','reader_id','persons'],['loans','book_title_id','book_titles'],
  ['loans','book_copy_id','book_copies'],['loans','created_terminal_id','terminals'],
  ['reservations','reader_id','persons'],['reservations','book_title_id','book_titles'],
  ['terminal_operations','terminal_id','terminals'],['idempotency_receipts','terminal_id','terminals'],['device_sync_state','terminal_id','terminals'],
 ];
 public function up(): void {
  foreach($this->parents as $table) DB::statement("ALTER TABLE {$table} ADD CONSTRAINT {$table}_id_school_unique UNIQUE (id,school_id)");
  foreach($this->relations as [$table,$column,$parent]) DB::statement("ALTER TABLE {$table} ADD CONSTRAINT {$table}_{$column}_scope FOREIGN KEY ({$column},school_id) REFERENCES {$parent}(id,school_id)");
  DB::statement('ALTER TABLE book_copies ADD CONSTRAINT copy_title_scope_unique UNIQUE (id,book_title_id,school_id)');
  DB::statement('ALTER TABLE loans ADD CONSTRAINT loan_copy_title_scope FOREIGN KEY (book_copy_id,book_title_id,school_id) REFERENCES book_copies(id,book_title_id,school_id)');
 }
 public function down(): void {
  DB::statement('ALTER TABLE loans DROP CONSTRAINT loan_copy_title_scope');
  DB::statement('ALTER TABLE book_copies DROP CONSTRAINT copy_title_scope_unique');
  foreach(array_reverse($this->relations) as [$table,$column]) DB::statement("ALTER TABLE {$table} DROP CONSTRAINT {$table}_{$column}_scope");
  foreach(array_reverse($this->parents) as $table) DB::statement("ALTER TABLE {$table} DROP CONSTRAINT {$table}_id_school_unique");
 }
};

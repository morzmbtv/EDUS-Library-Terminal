<?php

\Illuminate\Support\Facades\Artisan::command('edus:enrollment {school : Existing school UUID}', function () {
 $code=app(\App\Services\EnrollmentService::class)->issue($this->argument('school'));
 $this->line('Single-use code (valid for 24 hours): '.$code);
})->purpose('Issue a school-scoped terminal enrollment code to an infrastructure administrator');

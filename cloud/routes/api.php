<?php
use App\Http\Controllers\TerminalV1Controller;
use App\Http\Middleware\AuthenticateTerminal;
use Illuminate\Support\Facades\Route;
Route::get('/terminal/v1/health',[TerminalV1Controller::class,'health']);
Route::post('/terminal/v1/enroll',[TerminalV1Controller::class,'enroll'])->middleware('throttle:10,1');
Route::middleware(AuthenticateTerminal::class)->prefix('/terminal/v1')->group(function(){
 Route::post('/bootstrap',[TerminalV1Controller::class,'bootstrap']);
 Route::get('/changes',[TerminalV1Controller::class,'changes']);
 Route::post('/operations',[TerminalV1Controller::class,'operations']);
 Route::get('/operations/{operationId}',[TerminalV1Controller::class,'operation'])->whereUuid('operationId');
});

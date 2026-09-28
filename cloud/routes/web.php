<?php
use Illuminate\Support\Facades\Route;
use App\Http\Controllers\TerminalV1Controller;
// API infrastructure has no browser frontend or runtime CDN dependency.
Route::get('/', [TerminalV1Controller::class, 'health']);

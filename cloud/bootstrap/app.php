<?php

use Illuminate\Foundation\Application;
use Illuminate\Foundation\Configuration\Exceptions;
use Illuminate\Foundation\Configuration\Middleware;

return Application::configure(basePath: dirname(__DIR__))
    ->withRouting(
        web: __DIR__.'/../routes/web.php',
        api: __DIR__.'/../routes/api.php',
        commands: __DIR__.'/../routes/console.php',
        health: '/up',
    )
    ->withMiddleware(function (Middleware $middleware): void {
        //
    })
    ->withExceptions(function (Exceptions $exceptions): void {
        // QueryException can contain SQL bindings. Never report its message or trace.
        $exceptions->report(function (\Throwable $error) {
            \Illuminate\Support\Facades\Log::error('EDUS request failed', ['exception_type'=>get_class($error)]);
            return false;
        });
        $exceptions->render(function (\Throwable $error, \Illuminate\Http\Request $request) {
            if (!$request->is('api/terminal/*')) return null;
            if ($error instanceof \Illuminate\Validation\ValidationException || $error instanceof \Symfony\Component\HttpKernel\Exception\HttpExceptionInterface) return null;
            return response()->json(['success'=>false,'code'=>'SERVER_ERROR','message'=>'Сервер не выполнил запрос. Операция не подтверждена.'],500);
        });
    })->create();


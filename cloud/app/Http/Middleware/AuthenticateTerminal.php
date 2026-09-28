<?php
namespace App\Http\Middleware;
use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use Symfony\Component\HttpFoundation\Response;
final class AuthenticateTerminal {
 public function handle(Request $request, Closure $next): Response {
  $token=$request->bearerToken();
  if(!is_string($token)||$token==='') return response()->json(['success'=>false,'code'=>'DEVICE_UNAUTHORIZED','message'=>'Устройство не аутентифицировано.'],401);
  $device=DB::table('device_credentials as dc')->join('terminals as t','t.id','=','dc.terminal_id')->join('schools as s','s.id','=','t.school_id')->where('dc.token_hash',hash('sha256',$token))->select('dc.status as credential_status','t.id','t.school_id','t.status as terminal_status','t.protocol_version','s.status as school_status')->first();
  if(!$device) return response()->json(['success'=>false,'code'=>'DEVICE_UNAUTHORIZED','message'=>'Устройство не аутентифицировано.'],401);
  if($device->credential_status==='REVOKED'||$device->terminal_status==='REVOKED') return response()->json(['success'=>false,'code'=>'DEVICE_REVOKED','message'=>'Доступ устройства отозван.'],401);
  if($device->credential_status!=='ACTIVE'||$device->terminal_status!=='ACTIVE'||$device->school_status!=='ACTIVE') return response()->json(['success'=>false,'code'=>'DEVICE_UNAUTHORIZED','message'=>'Устройство недоступно.'],401);
  $school=$request->input('school_id',$request->query('school_id'));
  if($school!==null&&$school!==$device->school_id) return response()->json(['success'=>false,'code'=>'SCHOOL_SCOPE_VIOLATION','message'=>'Устройство не может обращаться к данным другой школы.'],403);
  $version=(string)$request->header('X-EDUS-Terminal-Version','');
  if(!preg_match('/^\d+\.\d+\.\d+$/',$version)||version_compare($version,config('library-cloud.minimum_terminal_version'),'<'))return response()->json(['success'=>false,'code'=>'TERMINAL_VERSION_UNSUPPORTED','message'=>'Обновите терминал.','minimum_terminal_version'=>config('library-cloud.minimum_terminal_version')],409);
  $request->attributes->set('edus.terminal',$device); return $next($request);
 }
}

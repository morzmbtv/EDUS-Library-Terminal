<?php
namespace Tests\Feature;
use Tests\TestCase;
final class OpenApiContractTest extends TestCase {
 public function test_openapi_documents_every_implemented_terminal_route(): void {
  $document=json_decode(file_get_contents(base_path('openapi/terminal-v1.openapi.json')),true,512,JSON_THROW_ON_ERROR);
  $this->assertSame('3.1.0',$document['openapi']);
  foreach(['/terminal/v1/health'=>'get','/terminal/v1/enroll'=>'post','/terminal/v1/bootstrap'=>'post','/terminal/v1/changes'=>'get','/terminal/v1/operations'=>'post','/terminal/v1/operations/{operationId}'=>'get'] as $path=>$method)$this->assertArrayHasKey($method,$document['paths'][$path]);
  $this->assertSame('1',config('library-cloud.sync_protocol_version'));$this->assertArrayHasKey('payload_json',$document['components']['schemas']['OperationBatch']['properties']['operations']['items']['properties']);
 }
}

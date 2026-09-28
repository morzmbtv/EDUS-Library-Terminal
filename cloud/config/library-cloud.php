<?php
return ['api_version'=>'v1','sync_protocol_version'=>'1','minimum_terminal_version'=>'1.1.0','cursor_secret'=>(env('SYNC_CURSOR_SECRET') ?: env('APP_KEY')),'max_batch_operations'=>50];

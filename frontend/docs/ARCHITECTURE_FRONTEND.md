# Responsibilities

`src/main.ts` mounts `Boot.vue`. Boot verifies backend identity, health and API compatibility before loading `App.vue`. Failures display a bounded, retrying unavailable state or an explicit incompatible-components message.

`App.vue` composes the existing agreed screen layout and global header. `src/shared/ui/` contains reusable touch controls. `src/pages/LibrarySearchView.vue` is the catalogue page. `src/features/terminal/useTerminal.ts` owns only scenario navigation, temporary reader/display state, scan queue and the uncommitted basket. It never changes operational records.

`src/shared/api/LocalServiceApiClient.ts` is the sole production API client. `localTransport.ts` owns transient HTTP session/CSRF context. API mutations and receipts remain in the backend. `src/shared/types/terminalTypes.ts` and `catalogSearch.ts` contain transport/read-model types and status labels, **not** a business engine.

HID framing preserves leading zeroes and letters; normalization/HMAC/database lookup are backend responsibilities. Face is an honest unavailable provider boundary; UAT preview uses browser getUserMedia and never stores images.

The frontend does not import backend source. `openapi/` is a versioned contract snapshot owned by the backend and checked by tests. Runtime assets are local. Temporary browser display caches can be lost safely; no persistent operational browser storage is used.

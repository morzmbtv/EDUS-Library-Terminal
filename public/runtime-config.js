/*
 * Site-specific, non-secret runtime settings.  The deployment process replaces
 * this file next to the static build; do not put credentials or tokens here.
 */
window.__EDUS_LIBRARY_RUNTIME_CONFIG__ = Object.freeze({
  apiBaseUrl: '',
  apiPrefix: '/api',
  language: 'ru',
  schoolId: '',
  deviceId: '',
  terminalType: 'LIBRARY',
  requestTimeoutMs: {
    read: 8000,
    write: 15000,
    reconcile: 8000,
  },
})

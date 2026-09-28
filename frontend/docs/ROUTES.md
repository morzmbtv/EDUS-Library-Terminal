# Controlled scenarios

Home has Issue, Return and Search. Issue/Return start with card identity, optionally switch to Face and return to Card, preserving operation context. Back uses scenario state, never blind browser history. Reader identification is required before scan/confirmation.

Search uses `#/library-search`. Details Back returns to results; results Back returns Home. Issue/reservation from a catalogue result returns to Search if identity is cancelled. Direct `#/issue` or `#/accept` links enter identification, not a fabricated reader session.

No registration, manual reader search, component gallery, student second-display route, Windows administrative route or dedicated Home navigation button is shipped.

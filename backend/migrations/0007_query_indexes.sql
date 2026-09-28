CREATE INDEX IF NOT EXISTS idx_copies_title_status ON book_copies(book_title_id,status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_loans_title_active_due ON loans(book_title_id,status,due_at);
CREATE INDEX IF NOT EXISTS idx_reservations_title_status ON reservations(book_title_id,status);
CREATE INDEX IF NOT EXISTS idx_cards_person ON cards(person_id);
CREATE INDEX IF NOT EXISTS idx_title_normalized_isbn ON book_titles(replace(replace(upper(isbn),'-',''),' ','')) WHERE deleted_at IS NULL;

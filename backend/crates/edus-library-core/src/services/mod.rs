use crate::{
    db::LocalDatabase,
    domain::{AppError, BasketItem, BookAvailability, ExpectedReturn, OperationResult, Title},
};

/// The only mutation entry point used by commands. Domain writes and outbox
/// insertion are one SQLite transaction inside LocalDatabase.
pub struct LibraryService;
impl LibraryService {
    pub fn issue(
        db: &mut LocalDatabase,
        reader: &str,
        items: &[BasketItem],
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        db.issue(reader, items, operation_id)
    }
    pub fn accept(
        db: &mut LocalDatabase,
        reader: &str,
        items: &[BasketItem],
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        db.accept(reader, items, operation_id)
    }
    pub fn reserve(
        db: &mut LocalDatabase,
        reader: &str,
        title: &str,
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        db.reserve(reader, title, operation_id)
    }
    pub fn availability(db: &LocalDatabase, title_id: &str) -> Result<BookAvailability, AppError> {
        db.book_availability(title_id)
    }
    pub fn expected_return(db: &LocalDatabase, title_id: &str) -> Result<ExpectedReturn, AppError> {
        db.nearest_expected_return(title_id)
    }
    pub fn cancel_reservation(
        db: &mut LocalDatabase,
        reader: &str,
        reservation_id: &str,
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        db.cancel_reservation(reader, reservation_id, operation_id)
    }
    pub fn register(
        db: &mut LocalDatabase,
        title: &Title,
        mode: &str,
        codes: &[String],
        quantity: i32,
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        db.register(title, mode, codes, quantity, operation_id)
    }
}

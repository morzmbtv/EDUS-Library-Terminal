//! Backend-owned catalogue read model. Availability derives from committed data.
use super::*;
use serde_json::{json, Value};
#[derive(Deserialize)]
pub(super) struct Search {
    query: String,
    #[serde(default = "default_sort")]
    sort: String,
}
fn default_sort() -> String {
    "relevance".into()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TitleRequest {
    title_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Cancel {
    reader_id: String,
    reservation_id: String,
    operation_id: String,
}
fn projection(snapshot: &Snapshot, title: &edus_library_core::domain::Title, today: &str) -> Value {
    let loans: Vec<_> = snapshot
        .loans
        .iter()
        .filter(|l| l.title_id == title.id)
        .collect();
    let copies:Vec<_>=snapshot.copies.iter().filter(|c|c.title_id==title.id).map(|c|{
        let loan=loans.iter().find(|l|l.copy_id.as_deref()==Some(&c.id));
        json!({"id":c.id,"code":c.code,"status":if loan.is_some(){"ON_LOAN"}else{&c.status},"location":c.location,"dueDate":loan.and_then(|l|l.due_date.as_ref())})
    }).collect();
    let available: Vec<_> = copies
        .iter()
        .filter(|c| c["status"] == "AVAILABLE")
        .map(|c| c["id"].clone())
        .collect();
    let dates: Vec<_> = loans.iter().filter_map(|l| l.due_date.as_deref()).collect();
    let legacy_total = *snapshot.legacy_stock.get(&title.id).unwrap_or(&0);
    let legacy_on_loan: i32 = loans
        .iter()
        .filter(|l| l.mode == "LEGACY_TITLE")
        .map(|l| l.quantity)
        .sum();
    json!({"title":title,"totalCopies":copies.iter().filter(|c|c["status"]!="WRITTEN_OFF").count(),"availableCopies":available.len(),"onLoanCopies":copies.iter().filter(|c|c["status"]=="ON_LOAN").count(),"legacyTotal":legacy_total,"legacyOnLoan":legacy_on_loan,"legacyAvailable":(legacy_total-legacy_on_loan).max(0),"queueCount":snapshot.reservations.iter().filter(|r|r.title_id==title.id&&r.status=="WAITING").count(),"copies":copies,"availableCopyIds":available,"nearestDueDate":dates.iter().filter(|d|**d>=today).min(),"hasOverdue":dates.iter().any(|d|*d<today)})
}
pub(super) async fn search(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<Search>,
) -> ApiResult<Vec<Value>> {
    session(&state, &headers, true)?;
    if input.query.len() > 512
        || !matches!(input.sort.as_str(), "title" | "relevance" | "available")
    {
        return Err(failure(
            "INVALID_INPUT",
            "Проверьте запрос поиска.",
            StatusCode::BAD_REQUEST,
        ));
    }
    with_runtime(&state, |runtime| {
        let snapshot = snapshot(runtime)?;
        let q = input
            .query
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let isbn: String = q
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut results: Vec<Value> = snapshot
            .titles
            .iter()
            .filter_map(|title| {
                let name = title.name.to_lowercase();
                let author = title.author.to_lowercase();
                let hay = format!(
                    "{} {} {} {}",
                    name,
                    author,
                    title.subject.to_lowercase(),
                    title.isbn.to_lowercase()
                );
                let exact = title.isbn.replace('-', "").to_lowercase() == isbn;
                if !exact && !q.split_whitespace().all(|w| hay.contains(w)) {
                    return None;
                }
                let score = if exact {
                    1000
                } else if name == q {
                    700
                } else if name.starts_with(&q) {
                    500
                } else if author == q {
                    450
                } else {
                    80
                };
                let mut result = projection(&snapshot, title, &today);
                result["relevance"] = json!(score);
                Some(result)
            })
            .collect();
        results.sort_by(|a, b| {
            let title_order = a["title"]["name"]
                .as_str()
                .cmp(&b["title"]["name"].as_str());
            match input.sort.as_str() {
                "title" => title_order,
                "available" => (b["availableCopies"].as_i64().unwrap_or(0)
                    + b["legacyAvailable"].as_i64().unwrap_or(0))
                .cmp(
                    &(a["availableCopies"].as_i64().unwrap_or(0)
                        + a["legacyAvailable"].as_i64().unwrap_or(0)),
                )
                .then(title_order),
                _ => b["relevance"]
                    .as_i64()
                    .cmp(&a["relevance"].as_i64())
                    .then(title_order),
            }
        });
        Ok(results)
    })
    .map(Json)
}
pub(super) async fn availability(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<TitleRequest>,
) -> ApiResult<Value> {
    session(&state, &headers, true)?;
    with_runtime(&state, |runtime| {
        let snapshot = snapshot(runtime)?;
        let title = snapshot
            .titles
            .iter()
            .find(|t| t.id == input.title_id)
            .ok_or_else(|| AppError::new("BOOK_NOT_FOUND", "Книга не найдена."))?;
        Ok(projection(
            &snapshot,
            title,
            &chrono::Utc::now().format("%Y-%m-%d").to_string(),
        ))
    })
    .map(Json)
}
pub(super) async fn face(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> ApiResult<Value> {
    session(&state, &headers, true)?;
    Err(failure(
        "FACE_PROVIDER_UNAVAILABLE",
        "Распознавание лица пока недоступно. Используйте карту.",
        StatusCode::SERVICE_UNAVAILABLE,
    ))
}
pub(super) async fn cancel(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<Cancel>,
) -> ApiResult<OperationResult> {
    session(&state, &headers, true)?;
    with_runtime(&state, |runtime| {
        selected(runtime, &input.reader_id)?;
        LibraryService::cancel_reservation(
            &mut *runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?,
            &input.reader_id,
            &input.reservation_id,
            &input.operation_id,
        )
    })
    .map(Json)
}

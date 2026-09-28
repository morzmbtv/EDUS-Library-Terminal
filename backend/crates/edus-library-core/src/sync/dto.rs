use crate::domain::AppError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct CloudChangeEnvelope {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub version: i64,
    pub operation: ChangeOperation,
    pub payload_kind: PayloadKind,
    pub payload_schema_version: u16,
    pub payload: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
// Variant suffixes intentionally mirror the canonical v1 wire vocabulary.
#[allow(clippy::enum_variant_names)]
pub(crate) enum EntityType {
    ClassUpsert,
    PersonUpsert,
    CardUpsert,
    FaceTemplateUpsert,
    LocationUpsert,
    BookTitleUpsert,
    BookCopyUpsert,
    LegacyStockUpsert,
    LoanUpsert,
    ReservationUpsert,
}
impl EntityType {
    pub(crate) fn wire_name(&self) -> &'static str {
        match self {
            Self::ClassUpsert => "CLASS_UPSERT",
            Self::PersonUpsert => "PERSON_UPSERT",
            Self::CardUpsert => "CARD_UPSERT",
            Self::FaceTemplateUpsert => "FACE_TEMPLATE_UPSERT",
            Self::LocationUpsert => "LOCATION_UPSERT",
            Self::BookTitleUpsert => "BOOK_TITLE_UPSERT",
            Self::BookCopyUpsert => "BOOK_COPY_UPSERT",
            Self::LegacyStockUpsert => "LEGACY_STOCK_UPSERT",
            Self::LoanUpsert => "LOAN_UPSERT",
            Self::ReservationUpsert => "RESERVATION_UPSERT",
        }
    }
    pub(crate) fn priority(&self) -> u8 {
        match self {
            Self::ClassUpsert => 0,
            Self::PersonUpsert => 1,
            Self::BookTitleUpsert => 2,
            Self::LocationUpsert => 3,
            Self::CardUpsert => 4,
            Self::FaceTemplateUpsert => 5,
            Self::BookCopyUpsert => 6,
            Self::LegacyStockUpsert => 7,
            Self::LoanUpsert => 8,
            Self::ReservationUpsert => 9,
        }
    }
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum ChangeOperation {
    Upsert,
    Delete,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum PayloadKind {
    Full,
    Tombstone,
}

/// The invariant present in every canonical entity DTO. Entity-specific fields
/// are checked below before the SQL projection is allowed to mutate state.
#[derive(Debug, Deserialize)]
struct CanonicalEntityHeader {
    id: String,
    school_id: String,
    version: i64,
}

pub(crate) struct ValidatedCloudChange {
    pub entity_type: String,
    pub entity_id: String,
    pub version: i64,
    pub delete: bool,
    pub payload: Value,
    pub priority: u8,
}

pub(crate) fn validate_change(
    value: &Value,
    expected_school_id: &str,
) -> Result<ValidatedCloudChange, AppError> {
    let envelope: CloudChangeEnvelope = serde_json::from_value(value.clone()).map_err(|error| {
        AppError::new(
            "CLOUD_PROTOCOL_ERROR",
            format!("Некорректный Cloud change: {error}"),
        )
    })?;
    if envelope.version < 1 || envelope.entity_id.trim().is_empty() {
        return Err(AppError::new(
            "CLOUD_PROTOCOL_ERROR",
            "Cloud change содержит некорректные id или version.",
        ));
    }
    if envelope.payload_schema_version != 1 {
        return Err(AppError::new(
            "CLOUD_PROTOCOL_UNSUPPORTED",
            "Версия DTO Cloud change не поддерживается.",
        ));
    }
    let delete = envelope.operation == ChangeOperation::Delete;
    if delete != (envelope.payload_kind == PayloadKind::Tombstone) {
        return Err(AppError::new(
            "CLOUD_PROTOCOL_ERROR",
            "operation и payload_kind Cloud change противоречат друг другу.",
        ));
    }
    let header: CanonicalEntityHeader =
        serde_json::from_value(envelope.payload.clone()).map_err(|_| {
            AppError::new(
                "CLOUD_DELTA_INCOMPLETE",
                "Cloud change не содержит канонический id, school_id и version.",
            )
        })?;
    if header.id != envelope.entity_id
        || header.version != envelope.version
        || header.school_id != expected_school_id
    {
        return Err(AppError::new(
            "SCHOOL_SCOPE_VIOLATION",
            "Cloud change не совпадает со scope терминала.",
        ));
    }
    if !delete {
        validate_full(envelope.entity_type.wire_name(), &envelope.payload)?;
    }
    Ok(ValidatedCloudChange {
        entity_type: envelope.entity_type.wire_name().into(),
        entity_id: envelope.entity_id,
        version: envelope.version,
        delete,
        payload: envelope.payload,
        priority: envelope.entity_type.priority(),
    })
}

fn validate_full(kind: &str, payload: &Value) -> Result<(), AppError> {
    let object = payload
        .as_object()
        .ok_or_else(|| AppError::new("CLOUD_DELTA_INCOMPLETE", "Cloud DTO должен быть object."))?;
    let required: &[&str] = match kind {
        "CLASS_UPSERT" => &["id", "school_id", "name", "version", "updated_at"],
        "PERSON_UPSERT" => &[
            "id",
            "school_id",
            "full_name",
            "person_type",
            "status",
            "version",
            "updated_at",
            "deleted_at",
        ],
        "CARD_UPSERT" => &[
            "id",
            "school_id",
            "person_id",
            "card_lookup_hash",
            "status",
            "version",
            "updated_at",
        ],
        "FACE_TEMPLATE_UPSERT" => &[
            "id",
            "school_id",
            "person_id",
            "template_ciphertext",
            "model_id",
            "model_version",
            "template_version",
            "consent_status",
            "enrolled_at",
            "updated_at",
        ],
        "LOCATION_UPSERT" => &["id", "school_id", "name", "code", "version", "updated_at"],
        "BOOK_TITLE_UPSERT" => &[
            "id",
            "school_id",
            "title",
            "authors",
            "language",
            "version",
            "updated_at",
            "deleted_at",
        ],
        "BOOK_COPY_UPSERT" => &[
            "id",
            "school_id",
            "book_title_id",
            "status",
            "version",
            "updated_at",
            "deleted_at",
        ],
        "LEGACY_STOCK_UPSERT" => &[
            "id",
            "school_id",
            "book_title_id",
            "total_quantity",
            "available_quantity",
            "version",
            "updated_at",
        ],
        "LOAN_UPSERT" => &[
            "id",
            "school_id",
            "reader_id",
            "book_title_id",
            "accounting_mode",
            "quantity",
            "returned_quantity",
            "issued_at",
            "status",
            "created_terminal_id",
            "version",
            "updated_at",
        ],
        "RESERVATION_UPSERT" => &[
            "id",
            "school_id",
            "reader_id",
            "book_title_id",
            "status",
            "version",
            "created_at",
            "updated_at",
        ],
        _ => &[],
    };
    if required.iter().any(|field| !object.contains_key(*field)) {
        return Err(AppError::new(
            "CLOUD_DELTA_INCOMPLETE",
            format!("{kind} не содержит полный канонический DTO."),
        ));
    }
    let allowed = match kind {
        "PERSON_UPSERT" => &["STUDENT", "TEACHER", "STAFF"][..],
        "CARD_UPSERT" => &["ACTIVE", "REVOKED"][..],
        "FACE_TEMPLATE_UPSERT" => &["ACTIVE", "REVOKED"][..],
        "BOOK_COPY_UPSERT" => &[
            "AVAILABLE",
            "ON_LOAN",
            "RESERVED",
            "REPAIR",
            "LOST",
            "WRITTEN_OFF",
            "VERIFYING",
        ][..],
        "LOAN_UPSERT" => &["ACTIVE", "RETURNED", "CANCELLED", "CONFLICT"][..],
        "RESERVATION_UPSERT" => &["WAITING", "FULFILLED", "CANCELLED"][..],
        _ => &[],
    };
    let status_field = if kind == "PERSON_UPSERT" {
        "person_type"
    } else if kind == "FACE_TEMPLATE_UPSERT" {
        "consent_status"
    } else {
        "status"
    };
    if !allowed.is_empty()
        && !object
            .get(status_field)
            .and_then(Value::as_str)
            .is_some_and(|value| allowed.contains(&value))
    {
        return Err(AppError::new(
            "CLOUD_PROTOCOL_ERROR",
            format!("{kind} содержит неизвестный {status_field}."),
        ));
    }
    Ok(())
}

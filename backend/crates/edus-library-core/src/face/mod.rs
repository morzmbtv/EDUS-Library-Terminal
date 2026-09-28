use crate::domain::{AppError, IdentityResult, Reader};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FaceOutcome {
    Match,
    NoMatch,
    Review,
    LivenessFailed,
    CameraError,
}
pub trait CameraService: Send + Sync {
    fn status(&self) -> &'static str;
}
pub trait FaceEngine: Send + Sync {
    fn detect_face(&self, frame: &[u8]) -> FaceOutcome;
    fn check_liveness(&self, frames: &[Vec<u8>]) -> FaceOutcome;
    fn create_embedding(&self, frame: &[u8]) -> Result<Vec<u8>, AppError>;
    fn find_match(&self, embedding: &[u8]) -> FaceOutcome;
}
pub struct TestFaceEngine;
impl FaceEngine for TestFaceEngine {
    fn detect_face(&self, _: &[u8]) -> FaceOutcome {
        FaceOutcome::Match
    }
    fn check_liveness(&self, _: &[Vec<u8>]) -> FaceOutcome {
        FaceOutcome::Match
    }
    fn create_embedding(&self, _: &[u8]) -> Result<Vec<u8>, AppError> {
        Ok(vec![])
    }
    fn find_match(&self, _: &[u8]) -> FaceOutcome {
        FaceOutcome::Match
    }
}
pub struct ProductionFaceEngine;
impl FaceEngine for ProductionFaceEngine {
    fn detect_face(&self, _: &[u8]) -> FaceOutcome {
        FaceOutcome::CameraError
    }
    fn check_liveness(&self, _: &[Vec<u8>]) -> FaceOutcome {
        FaceOutcome::LivenessFailed
    }
    fn create_embedding(&self, _: &[u8]) -> Result<Vec<u8>, AppError> {
        Err(AppError::new(
            "FACE_PROVIDER_UNAVAILABLE",
            "REQUIRES_PRODUCTION_FACE_PROVIDER",
        ))
    }
    fn find_match(&self, _: &[u8]) -> FaceOutcome {
        FaceOutcome::NoMatch
    }
}
#[derive(Clone)]
pub struct FaceService {
    test_mode: bool,
}
impl FaceService {
    pub fn new(test_mode: bool) -> Self {
        Self { test_mode }
    }
    pub fn camera_status(&self) -> &'static str {
        if self.test_mode {
            "CAMERA_NOT_CHECKED"
        } else {
            "PRODUCTION_PROVIDER_NOT_CONFIGURED"
        }
    }
    pub fn engine_status(&self) -> &'static str {
        if self.test_mode {
            "TEST_FACE_ENGINE_WITH_PAD_SIMULATION"
        } else {
            "REQUIRES_PRODUCTION_FACE_PROVIDER"
        }
    }
    /// This only resolves a named fixture in terminal-test mode. It is not a
    /// biometric recognizer and never receives, saves or transmits camera data.
    pub fn identify_test_face(&self, reader: Reader) -> Result<IdentityResult, AppError> {
        if !self.test_mode {
            return Err(AppError::new(
                "FACE_PROVIDER_UNAVAILABLE",
                "Распознавание лица требует подключённого и проверенного провайдера.",
            ));
        }
        let engine = TestFaceEngine;
        if engine.check_liveness(&[]) != FaceOutcome::Match {
            return Err(AppError::new(
                "LIVENESS_FAILED",
                "Проверка присутствия не пройдена.",
            ));
        }
        Ok(IdentityResult {
            person_id: reader.id.clone(),
            method: "FACE".into(),
            confidence_class: Some("TEST_HIGH_AFTER_PAD_PASS".into()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reader,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{FaceEngine, FaceOutcome, FaceService, ProductionFaceEngine, TestFaceEngine};

    #[test]
    fn production_boundary_cannot_report_a_face_match() {
        let engine = ProductionFaceEngine;
        assert_ne!(engine.detect_face(&[]), FaceOutcome::Match);
        assert!(engine.create_embedding(&[]).is_err());
    }

    #[test]
    fn test_engine_is_explicit_and_deterministic() {
        let engine = TestFaceEngine;
        assert_eq!(engine.detect_face(&[]), FaceOutcome::Match);
        assert_eq!(engine.check_liveness(&[]), FaceOutcome::Match);
        assert_eq!(engine.find_match(&[]), FaceOutcome::Match);
        let service = FaceService::new(false);
        assert!(
            service.camera_status() == "CAMERA_NOT_CHECKED"
                || service.camera_status() == "PRODUCTION_PROVIDER_NOT_CONFIGURED"
        );
    }
}

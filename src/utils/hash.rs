use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Utilidades para hashing
pub struct HashHelper;

impl HashHelper {
    /// Calcular hash simple de un string
    pub fn hash_string(s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Calcular hash de configuración
    pub fn hash_config<T: serde::Serialize>(config: &T) -> Result<String, String> {
        let json = serde_json::to_string(config)
            .map_err(|e| format!("Error serializando: {}", e))?;
        
        Ok(format!("{:x}", Self::hash_string(&json)))
    }
    
    /// Verificar si dos configuraciones son iguales por hash
    pub fn configs_equal<T: serde::Serialize>(a: &T, b: &T) -> bool {
        Self::hash_config(a).ok() == Self::hash_config(b).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_string() {
        let hash1 = HashHelper::hash_string("test");
        let hash2 = HashHelper::hash_string("test");
        let hash3 = HashHelper::hash_string("different");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
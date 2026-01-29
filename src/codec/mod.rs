//! Módulo de codificación/decodificación de medios
//! 
//! Maneja la configuración y parámetros de codificación de video y audio
//! para diferentes estrategias de transcodificación

pub mod video;
pub mod audio;
pub mod encoder;
pub mod decoder;

pub use video::VideoEncoder;
pub use audio::AudioEncoder;
pub use encoder::Encoder;
pub use decoder::Decoder;

//! Módulo de multiplexing/demultiplexing MPEG-TS

pub mod mpegts_demuxer;
pub mod mpegts_muxer;
pub mod packet;
pub mod pes;
pub mod psi;

pub use mpegts_demuxer::MpegTsDemuxer;
pub use mpegts_muxer::MpegTsMuxer;
pub use packet::{TsPacket, TsHeader, AdaptationField};
pub use pes::PesPacket;
pub use psi::{Pat, Pmt, ProgramInfo};
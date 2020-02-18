//#![doc(html_logo_url = "http://kekbit.com/logo.jpg")]

pub mod codecs {
    pub use kekbit_codecs::codecs::raw::*;
    pub use kekbit_codecs::codecs::text::*;
    pub use kekbit_codecs::codecs::DataFormat;
    pub use kekbit_codecs::codecs::Encodable;
}
pub mod core {
    pub use kekbit_core::api::*;
    pub use kekbit_core::header::*;
    pub use kekbit_core::shm::reader::ShmReader;
    pub use kekbit_core::shm::shm_reader;
    pub use kekbit_core::shm::shm_writer;
    pub use kekbit_core::shm::storage_path;
    pub use kekbit_core::shm::try_shm_reader;
    pub use kekbit_core::shm::writer::ShmWriter;
    pub use kekbit_core::tick::*;
}

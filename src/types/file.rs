use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq , Serialize, Deserialize)]
pub enum FileType {
    Image(ImageType),
    Document(DocumentType),
    Video(VideoType),
    Audio(AudioType),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageType {
    Jpeg,
    Png,
    Gif,
    Webp,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DocumentType {
    Pdf,
    Doc,
    Docx,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VideoType {
    Mp4,
    Mkv,
    Avi,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioType {
    Mp3,
    Wav,
    Flac,
    Other(String),
}

pub struct FileTypeDetector;

impl FileTypeDetector {
    pub fn detect(data: &[u8]) -> FileType {
        if let Some(kind) = infer::get(data) {
            match kind.mime_type() {
                // Image types
                "image/jpeg" => FileType::Image(ImageType::Jpeg),
                "image/png" => FileType::Image(ImageType::Png),
                "image/gif" => FileType::Image(ImageType::Gif),
                "image/webp" => FileType::Image(ImageType::Webp),
                
                // Document types
                "application/pdf" => FileType::Document(DocumentType::Pdf),
                "application/msword" => FileType::Document(DocumentType::Doc),
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => 
                    FileType::Document(DocumentType::Docx),
                
                // Video types
                "video/mp4" => FileType::Video(VideoType::Mp4),
                "video/x-matroska" => FileType::Video(VideoType::Mkv),
                "video/x-msvideo" => FileType::Video(VideoType::Avi),
                
                // Audio types
                "audio/mpeg" => FileType::Audio(AudioType::Mp3),
                "audio/wav" => FileType::Audio(AudioType::Wav),
                "audio/flac" => FileType::Audio(AudioType::Flac),
                
                // Other types
                mime if mime.starts_with("image/") => 
                    FileType::Image(ImageType::Other(mime.to_string())),
                mime if mime.starts_with("video/") => 
                    FileType::Video(VideoType::Other(mime.to_string())),
                mime if mime.starts_with("audio/") => 
                    FileType::Audio(AudioType::Other(mime.to_string())),
                mime if mime.starts_with("application/") => 
                    FileType::Document(DocumentType::Other(mime.to_string())),
                _ => FileType::Unknown,
            }
        } else {
            FileType::Unknown
        }
    }
}

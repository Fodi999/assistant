// ─── SEO Module — decomposed template generation ────────────────────────────
//
// Split from the 934-line templates.rs monolith into focused files:
//   title.rs       — SEO title generation (≤60 chars)
//   description.rs — Meta description (80-155 chars)
//   headings.rs    — H1 + intro paragraph (featured snippets)
//   faq.rs         — FAQ schema generation
//   content.rs     — why_it_works, how_to_cook, optimization_tips
//   quality.rs     — page quality scoring (0-100)
//   helpers.rs     — capitalize_words, smart_truncate

pub mod title;
pub mod description;
pub mod headings;
pub mod faq;
pub mod content;
pub mod quality;
pub mod helpers;

// Re-export the main API
pub use title::generate_title;
pub use description::generate_description;
pub use headings::{generate_h1, generate_intro};
pub use faq::generate_faq;
pub use content::{generate_why_it_works, generate_how_to_cook, generate_optimization_tips};
pub use quality::{quality_score, page_quality_score};
pub use helpers::{capitalize_words, smart_truncate};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Unified extraction block shared by both sidecar engines and rendered as
/// Markdown on the client.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PdfBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<Vec<f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

impl PdfBlock {
    pub fn is_text(&self) -> bool {
        matches!(self.block_type.as_str(), "heading" | "paragraph")
            && self
                .text
                .as_deref()
                .map(|text| !text.trim().is_empty())
                .unwrap_or(false)
    }
}

/// Render the block schema as Markdown. The saturated image data URIs are
/// intentionally embedded so a cached extraction is self-contained.
pub fn blocks_to_markdown(blocks: &[PdfBlock]) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        match block.block_type.as_str() {
            "heading" => {
                let Some(text) = block
                    .text
                    .as_deref()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                else {
                    continue;
                };
                let level = block.level.unwrap_or(2).clamp(1, 6);
                parts.push(format!("{} {text}", "#".repeat(level as usize)));
            }
            "paragraph" => {
                if let Some(text) = block
                    .text
                    .as_deref()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                {
                    parts.push(text.to_string());
                }
            }
            "image" | "table_image" | "formula_image" => {
                if let Some(image) = block.image.as_deref().filter(|image| !image.is_empty()) {
                    let label = block.block_type.trim_end_matches("_image");
                    parts.push(format!("![{label}]({image})"));
                }
            }
            _ => {}
        }
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(block_type: &str, text: Option<&str>) -> PdfBlock {
        PdfBlock {
            block_type: block_type.to_string(),
            text: text.map(str::to_string),
            level: None,
            page: None,
            bbox: None,
            image: None,
        }
    }

    #[test]
    fn renders_all_block_types() {
        let mut image = block("image", None);
        image.image = Some("data:image/png;base64,aaa".to_string());
        let mut table = block("table_image", None);
        table.image = Some("data:image/png;base64,bbb".to_string());
        let mut formula = block("formula_image", None);
        formula.image = Some("data:image/png;base64,ccc".to_string());
        let mut heading = block("heading", Some("Calculus"));
        heading.level = Some(1);

        let markdown = blocks_to_markdown(&[
            heading,
            block("paragraph", Some("Limits.")),
            image,
            table,
            formula,
        ]);

        assert_eq!(
            markdown,
            "# Calculus\n\nLimits.\n\n![image](data:image/png;base64,aaa)\n\n\
             ![table](data:image/png;base64,bbb)\n\n![formula](data:image/png;base64,ccc)"
        );
    }

    #[test]
    fn clamps_heading_level_and_skips_empty_text() {
        let mut heading = block("heading", Some("Deep"));
        heading.level = Some(42);
        assert_eq!(blocks_to_markdown(&[heading]), "###### Deep");
        assert_eq!(blocks_to_markdown(&[block("paragraph", Some("  "))]), "");
    }
}

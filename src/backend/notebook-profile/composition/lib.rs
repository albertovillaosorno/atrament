// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
//
// Boundary-Contract:
// - Owns:
//   - Versioned notebook page metadata composed with Rust-canonical paper.
// - Must-Not:
//   - Claim that date text, borders, or page numbers are rendered.
// - Allows:
//   - Inputs: Bounded notebook JSON and admitted native paper profile JSON.
//   - Outputs: Canonical notebook JSON or explicit typed-field refusal.
//   - Side effects: None.
// - Split-When:
//   - Notebook layout/rendering integrates these metadata settings.
// - Merge-When:
//   - Full native notebook profile serialization absorbs this slice.
// - Summary:
//   - Builds the first composite notebook profile without another paper codec.
// - Description:
//   - Validates date, border and printed-page settings with paper geometry.
// - Usage:
//   - NotebookProfile::from_json and NotebookProfile::canonical_json.
// - Defaults:
//   - A4 grid with no date, border rounding, page numbers or printed flags.
//

//! Rust-owned notebook metadata and the canonical nested paper profile.

use atrament_notebook_paper_cli::{
    PaperDatePlaceholder, canonical_paper_profile_json,
    default_paper_profile_json, paper_profile_pdf_pages_with_date,
    paper_profile_png_layer_with_date, validate_paper_date_clearance,
};
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxLimits, ValidatedProfileJsonObject,
    ValidatedProfileJsonValue, validate_profile_json_object,
};

/// Upper bound on an admitted notebook JSON file, including whitespace.
pub const MAX_NOTEBOOK_JSON_BYTES: usize = 8_192;
const SCHEMA: &str = "atrament.notebook.v1";

/// How a future notebook renderer will draw the date placeholder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DateStyle {
    /// A box reserved for the date.
    Box,
    /// A line to write a date on.
    Line,
    /// No printed date placeholder.
    None,
}

impl DateStyle {
    /// Canonical JSON/CLI spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Line => "line",
            Self::Box => "box",
        }
    }

    /// Interpret one explicit style.
    ///
    /// # Errors
    /// Refuses unrecognized styles.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "none" => Ok(Self::None),
            "line" => Ok(Self::Line),
            "box" => Ok(Self::Box),
            _ => Err("date style must be none, line or box".into()),
        }
    }
}

/// Reserved top edge for an optional date placeholder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatePlacement {
    /// No printed date placeholder.
    None,
    /// Reserve a placeholder at the top left.
    TopLeft,
    /// Reserve a placeholder at the top right.
    TopRight,
}

impl DatePlacement {
    /// Canonical JSON/CLI spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
        }
    }

    /// Interpret one explicit placement.
    ///
    /// # Errors
    /// Refuses unrecognized placements.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "none" => Ok(Self::None),
            "top-left" => Ok(Self::TopLeft),
            "top-right" => Ok(Self::TopRight),
            _ => {
                Err("date placement must be none, top-left or top-right".into())
            }
        }
    }
}

/// One editable notebook profile, with the existing Rust paper authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotebookProfile {
    /// Requested corner radius for a future notebook border, in micrometres.
    pub border_radius_um: u32,
    /// Reserved date placeholder position.
    pub date_placement: DatePlacement,
    /// Reserved date placeholder style.
    pub date_style: DateStyle,
    /// Whether a future renderer should show page numbers.
    pub page_numbers: bool,
    /// Canonical `atrament.paper.v1` JSON; no second page codec is invented.
    pub paper_json: String,
    /// Whether the sheet already has its own physical printed markings.
    pub printed_sheet: bool,
}

impl Default for NotebookProfile {
    fn default() -> Self {
        Self {
            paper_json: default_paper_profile_json(),
            date_style: DateStyle::None,
            date_placement: DatePlacement::None,
            border_radius_um: 0,
            page_numbers: false,
            printed_sheet: false,
        }
    }
}

impl NotebookProfile {
    /// Return canonical notebook JSON with canonical embedded paper.
    ///
    /// # Errors
    /// Refuses invalid profile settings before emitting any bytes.
    pub fn canonical_json(&self) -> Result<String, String> {
        self.validate()?;
        let paper = canonical_paper_profile_json(self.paper_json.as_bytes())?;
        let json = format!(
            concat!(
                "{{\n",
                "  \"schema\": \"atrament.notebook.v1\",\n",
                "  \"paper\": {paper},\n",
                "  \"date_style\": \"{date_style}\",\n",
                "  \"date_placement\": \"{date_placement}\",\n",
                "  \"border_radius_um\": {border_radius_um},\n",
                "  \"page_numbers\": {page_numbers},\n",
                "  \"printed_sheet\": {printed_sheet}\n",
                "}}\n"
            ),
            paper = paper.trim_end().replace('\n', "\n  "),
            date_style = self.date_style.key(),
            date_placement = self.date_placement.key(),
            border_radius_um = self.border_radius_um,
            page_numbers = self.page_numbers,
            printed_sheet = self.printed_sheet,
        );
        if json.len() > MAX_NOTEBOOK_JSON_BYTES {
            return Err("canonical notebook JSON too large".into());
        }
        Ok(json)
    }
    const fn date_placeholder(&self) -> Option<PaperDatePlaceholder> {
        match (self.date_style, self.date_placement) {
            (DateStyle::Line, DatePlacement::TopLeft) => {
                Some(PaperDatePlaceholder::LeftLine)
            }
            (DateStyle::Box, DatePlacement::TopLeft) => {
                Some(PaperDatePlaceholder::LeftBox)
            }
            (DateStyle::Line, DatePlacement::TopRight) => {
                Some(PaperDatePlaceholder::RightLine)
            }
            (DateStyle::Box, DatePlacement::TopRight) => {
                Some(PaperDatePlaceholder::RightBox)
            }
            _ => None,
        }
    }

    /// Decode a complete bounded notebook profile without dropping fields.
    ///
    /// # Errors
    /// Refuses unknown/duplicate fields, types, geometry and versions.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let root = parse_root(bytes)?;
        if root.root_fields().count() != 7 {
            return Err("notebook requires exactly seven known fields".into());
        }
        if string_member(root, "schema")? != SCHEMA {
            return Err("unsupported notebook profile schema".into());
        }
        let paper = root
            .root_member_value("paper")
            .ok_or("missing notebook paper profile")?;
        let profile = Self {
            paper_json: canonical_paper_profile_json(paper.bytes())?,
            date_style: DateStyle::parse(&string_member(root, "date_style")?)?,
            date_placement: DatePlacement::parse(&string_member(
                root,
                "date_placement",
            )?)?,
            border_radius_um: number_member(root, "border_radius_um")?,
            page_numbers: boolean_member(root, "page_numbers")?,
            printed_sheet: boolean_member(root, "printed_sheet")?,
        };
        profile.validate()?;
        Ok(profile)
    }

    /// Export physical PDF paper layers from a valid notebook profile.
    ///
    /// A selected empty date box/line is drawn in the top margin; actual date
    /// text, borders, page numbers, images and handwriting are not rendered.
    /// Preprinted stationery suppresses ruling but keeps the date placeholder.
    ///
    /// # Errors
    /// Refuses invalid metadata, geometry, page count or output limits.
    pub fn paper_layer_pdf(&self, pages: usize) -> Result<Vec<u8>, String> {
        self.validate()?;
        paper_profile_pdf_pages_with_date(
            self.paper_json.as_bytes(), pages, !self.printed_sheet,
            self.date_placeholder(),
        )
    }

    /// Export a transparent RGBA paper layer at a bounded physical DPI.
    ///
    /// # Errors
    /// Refuses invalid metadata, paper geometry, DPI or raster budgets.
    pub fn paper_layer_png(&self, dpi: u32) -> Result<Vec<u8>, String> {
        self.validate()?;
        paper_profile_png_layer_with_date(
            self.paper_json.as_bytes(), dpi, !self.printed_sheet,
            self.date_placeholder(),
        )
    }

    /// Check all fields, including border bounds against the printable area.
    ///
    /// # Errors
    /// Refuses contradictory date fields, impossible radius or invalid paper.
    pub fn validate(&self) -> Result<(), String> {
        if (self.date_style == DateStyle::None)
            != (self.date_placement == DatePlacement::None)
        {
            return Err(
                "date style and placement must both be none or set".into()
            );
        }
        let paper = canonical_paper_profile_json(self.paper_json.as_bytes())?;
        if self.date_placeholder().is_some() {
            validate_paper_date_clearance(paper.as_bytes())?;
        }
        let root = parse_root(paper.as_bytes())?;
        let measure = |key: &str| -> Result<u64, String> {
            root.root_member_value(key)
                .and_then(ValidatedProfileJsonValue::unsigned_integer)
                .ok_or_else(|| format!("invalid paper measurement: {key}"))
        };
        let inner_width = measure("width_um")?
            .checked_sub(
                measure("margin_left_um")?
                    .checked_add(measure("margin_right_um")?)
                    .ok_or("paper horizontal margins overflow")?,
            )
            .ok_or("paper margins exceed sheet width")?;
        let inner_height = measure("height_um")?
            .checked_sub(
                measure("margin_top_um")?
                    .checked_add(measure("margin_bottom_um")?)
                    .ok_or("paper vertical margins overflow")?,
            )
            .ok_or("paper margins exceed sheet height")?;
        if u64::from(self.border_radius_um)
            .checked_mul(2)
            .is_none_or(|diameter| diameter > inner_width.min(inner_height))
        {
            return Err("border radius exceeds half the printable area".into());
        }
        Ok(())
    }
}

fn boolean_member(
    root: ValidatedProfileJsonObject<'_>,
    name: &str,
) -> Result<bool, String> {
    root.root_member_value(name)
        .and_then(ValidatedProfileJsonValue::boolean)
        .ok_or_else(|| format!("notebook {name} requires a JSON boolean"))
}

fn number_member(
    root: ValidatedProfileJsonObject<'_>,
    name: &str,
) -> Result<u32, String> {
    let value = root
        .root_member_value(name)
        .and_then(ValidatedProfileJsonValue::unsigned_integer)
        .ok_or_else(|| {
            format!("notebook {name} requires integer micrometres")
        })?;
    u32::try_from(value)
        .map_err(|error| format!("notebook {name} is too large: {error}"))
}

fn parse_root(bytes: &[u8]) -> Result<ValidatedProfileJsonObject<'_>, String> {
    validate_profile_json_object(
        bytes,
        ProfileJsonSyntaxLimits {
            maximum_bytes: MAX_NOTEBOOK_JSON_BYTES,
            maximum_nesting: 3,
            maximum_object_members: 16,
            maximum_values: 40,
        },
    )
    .map_err(|error| format!("notebook JSON refused: {error:?}"))
}

fn string_member(
    root: ValidatedProfileJsonObject<'_>,
    name: &str,
) -> Result<String, String> {
    let member = root
        .root_member_value(name)
        .ok_or_else(|| format!("missing notebook field: {name}"))?;
    let mut buffer = [0u8; 80];
    let count = member
        .copy_string_utf8(&mut buffer)
        .map_err(|error| format!("notebook {name} refused: {error:?}"))?;
    String::from_utf8(
        buffer
            .get(..count)
            .ok_or("notebook string overflow")?
            .to_vec(),
    )
    .map_err(|error| format!("notebook {name} invalid UTF-8: {error}"))
}

#[cfg(test)]
#[path = "../../../../tests/backend/notebook-profile/composition/lib.rs"]
mod tests;

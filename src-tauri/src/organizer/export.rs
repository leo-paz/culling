//! Photo export: copy keepers to output folder, organize by person, trash rejects.

use crate::error::CullingError;
use crate::project::{Grade, Project};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize)]
pub enum GradeFilter {
    All,
    OkAndGood,
    GoodOnly,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum Organization {
    Flat,
    ByPerson,
}

pub struct ExportOptions {
    pub output_dir: PathBuf,
    pub grade_filter: GradeFilter,
    pub organization: Organization,
    pub trash_bad: bool,
}

/// Export photos according to the given options.
/// Returns the number of photos exported.
pub fn export_photos<F>(
    project: &Project,
    options: &ExportOptions,
    on_progress: F,
) -> Result<usize, CullingError>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let output = &options.output_dir;
    fs::create_dir_all(output)?;

    // Filter photos by grade
    // Ungraded photos count as "Ok" for filtering purposes
    let photos: Vec<_> = project
        .photos
        .iter()
        .filter(|p| match options.grade_filter {
            GradeFilter::All => true,
            GradeFilter::OkAndGood => {
                matches!(p.grade, Grade::Ok | Grade::Good | Grade::Ungraded)
            }
            GradeFilter::GoodOnly => matches!(p.grade, Grade::Good),
        })
        .collect();

    let total = photos.len();

    for (i, photo) in photos.iter().enumerate() {
        let dest = match options.organization {
            Organization::Flat => output.join(&photo.filename),
            Organization::ByPerson => {
                // Find the primary cluster for this photo (first face's cluster)
                let cluster_label = photo
                    .faces
                    .first()
                    .and_then(|f| f.cluster_id)
                    .and_then(|cid| project.clusters.iter().find(|c| c.id == cid))
                    .map(|c| c.label.clone())
                    .unwrap_or_else(|| "Ungrouped".to_string());

                let person_dir = output.join(&cluster_label);
                fs::create_dir_all(&person_dir)?;
                person_dir.join(&photo.filename)
            }
        };

        fs::copy(&photo.path, &dest)?;
        on_progress(i + 1, total);
    }

    // Trash bad photos if requested
    if options.trash_bad {
        let bad_photos: Vec<_> = project
            .photos
            .iter()
            .filter(|p| p.grade == Grade::Bad)
            .collect();
        for photo in &bad_photos {
            if let Err(e) = trash::delete(&photo.path) {
                eprintln!(
                    "Warning: failed to trash {}: {}",
                    photo.filename, e
                );
            }
        }
    }

    Ok(total)
}

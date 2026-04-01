//! Photo export: copy keepers to output folder, organize by person, trash rejects.

use crate::project::{Grade, Project};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum GradeFilter {
    All,
    OkAndGood,
    GoodOnly,
}

#[derive(Debug, Clone)]
pub enum Organization {
    Flat,
    ByPerson,
}

pub struct ExportOptions {
    pub output_dir: String,
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
) -> Result<usize, String>
where
    F: Fn(usize, usize),
{
    let output = Path::new(&options.output_dir);
    fs::create_dir_all(output).map_err(|e| format!("Failed to create output directory: {}", e))?;

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
                fs::create_dir_all(&person_dir).map_err(|e| {
                    format!("Failed to create person directory '{}': {}", cluster_label, e)
                })?;
                person_dir.join(&photo.filename)
            }
        };

        fs::copy(&photo.path, &dest)
            .map_err(|e| format!("Failed to copy {}: {}", photo.filename, e))?;
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

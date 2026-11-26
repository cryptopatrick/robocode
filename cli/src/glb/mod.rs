//! Export a battle recording as a glTF 2.0 binary (`.glb`). See `_dev/PRD.md` 5.7.

pub mod geometry;
pub mod replay;
pub mod scene;
pub mod space;
pub mod tanks;
pub mod writer;

/// Turns a recording (JSON) into `.glb` bytes.
pub fn export(recording_json: &str) -> Result<Vec<u8>, String> {
    let replay = replay::parse(recording_json)?;
    Ok(writer::write_glb(&scene::build(&replay)))
}

//! Sound effects: a procedural synth, and a `PlaySound` message any feature can send to play one.
//! Features build their own sounds (see `ball/sound.rs`, `cat/sound.rs`, `goal/sound.rs`).

mod synth;

use bevy::audio::{AddAudioSource, Volume};
use bevy::prelude::*;

pub use synth::Synth;

/// Volume of every sound effect, from 0 (muted) to 1 (as rendered).
const MASTER_VOLUME: f32 = 0.8;
/// Most sounds started in one frame; extras are dropped so a pile-up can't flood the mixer.
const MAX_SOUNDS_PER_FRAME: usize = 8;

/// Plugin that registers the synth as an audio source and plays requested sounds.
pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<Synth>()
            .add_message::<PlaySound>()
            .add_systems(PostUpdate, play_sounds);
    }
}

/// Send to play a synth sound once.
#[derive(Message, Clone, Debug)]
pub struct PlaySound(pub Synth);

/// Spawns a self-despawning audio player for each requested sound.
fn play_sounds(
    mut commands: Commands,
    mut requests: MessageReader<PlaySound>,
    mut synths: ResMut<Assets<Synth>>,
) {
    for (i, PlaySound(synth)) in requests.read().enumerate() {
        if i >= MAX_SOUNDS_PER_FRAME {
            continue;
        }
        commands.spawn((
            Name::new("Sound"),
            AudioPlayer(synths.add(*synth)),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(MASTER_VOLUME)),
        ));
    }
}

use midly::{MidiMessage, live::LiveEvent};
use simonor::{MidiReceiver, SharedMidiSender};

pub struct MidiProcessor {
    midi_sender: SharedMidiSender,
}

impl MidiProcessor {
    pub fn new(midi_sender: SharedMidiSender) -> Self {
        Self { midi_sender, }
    }
}

impl MidiReceiver for MidiProcessor {
    fn on_message(&self, message: &[u8]) {
        let raw_message = message;
        let mut midi_sender= self.midi_sender.lock().unwrap();
        let event = LiveEvent::parse(message).unwrap();
        if let LiveEvent::Midi { channel, message } = event {
            match message {
                MidiMessage::Controller { controller, value } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 != 16 {
                        return;
                    }
                }
                MidiMessage::Aftertouch { key, vel } => {
                //     let channel1 = u8::from(channel) + 1; // 1-based channel number.
                //     if channel1 == 16 {
                //         // When the editor requests a preset load, which can be seen in the
                //         // editor's console log but not here, the program number is zero-based.
                //         // When the instrument confirms that the preset has been loaded,
                //         // which we see here, the program number is one-based.
                //         trace!("on_message_received: ProgramChange ch16 program {}", u8::from(program));
                //         let download_status = *self.download_status.lock().unwrap();
                //         // I don't think this will work if system presets are downloaded.
                //         // But it's a rare occurrence; and the user will be able to work around it.
                //         if download_status == DownloadStatus::EndUserNames
                //             || download_status == DownloadStatus::EndSysNames {
                //             trace!("on_message_received: End of download");
                //             self.on_init_data_download_completed();
                //             return;
                //         }
                //         if download_status == DownloadStatus::Complete {
                //             // The user is selecting a preset. The editor sends the preset's
                //             // zero-based program number after the bank.
                //             // For unknown reason, this happens twice when a preset is loaded
                //             // from disc.
                //             trace!("on_message_received: Program, preset selected");
                //             if let Some(listener) = self.listener() {
                //                 rayon::spawn(move || listener.on_new_preset_selected());
                //             }
                //         }
                //     }
                }
                _ => {
                    midi_sender.send_message(raw_message, 0);
                }
            }
        }
    }
}
use midly::{MidiMessage, live::LiveEvent};
use simonor::{MidiReceiver, SharedMidiSender};
use crate::{CcNoRangeDefaults, CcNoRangeSetters};

pub struct MidiProcessor {
    down_cc_no: u8,
    up_cc_no: u8,
    in_cc_no: u8,
    out_cc_no: u8,
    octave_shift: i8,
    midi_sender: SharedMidiSender,
}

impl MidiProcessor {
    pub fn new(midi_sender: SharedMidiSender) -> Self {
        Self {
            down_cc_no: CcNoRangeDefaults::down_cc_no(),
            up_cc_no: CcNoRangeDefaults::up_cc_no(),
            in_cc_no: CcNoRangeDefaults::in_cc_no(),
            out_cc_no: CcNoRangeDefaults::out_cc_no(),
            octave_shift: 0,
            midi_sender,
        }
    }
    
    fn get_message_to_send<'a>(&mut self, raw_message: &'a [u8]) -> Option<&'a [u8]> {
        let event = LiveEvent::parse(raw_message).unwrap();
        if let LiveEvent::Midi { channel, message } = event {
            match message {
                MidiMessage::Controller { controller, value } => {
                     match controller {
                        controller if controller == self.down_cc_no => {
                            if value == 127 {
                                if self.octave_shift > -3 {
                                    self.octave_shift -= 1;
                                }
                            }
                            None
                        }
                        controller if controller == self.up_cc_no => {
                            if value == 127 {
                                if self.octave_shift < 3 {
                                    self.octave_shift += 1;
                                }
                            }
                            None
                        }
                        controller if controller == self.in_cc_no => {
                            if self.in_cc_no != self.out_cc_no {
                                return
                            }
                            Some(raw_message)
                        }
                        controller if controller == self.out_cc_no => {
                            None
                        }
                        _ => {
                            Some(raw_message)
                        }
                    }
                }
                MidiMessage::NoteOn { key, vel } => {
                    Some(raw_message)
                }
                MidiMessage::NoteOff { key, vel } => {
                    Some(raw_message)
                }
                MidiMessage::Aftertouch { key, vel } => {
                    Some(raw_message)
                }
                _ => {
                    Some(raw_message)
                }
            }
        } else {
            // A System Common message, such as System Exclusive,
            // or a one-byte System Realtime message.
            Some(raw_message)
        }
    }
}

impl CcNoRangeSetters for MidiProcessor {
    fn set_down_cc_no(&mut self, value: u8) {
        self.down_cc_no = value;
    }

    fn set_up_cc_no(&mut self, value: u8) { self.up_cc_no = value; }

    fn set_in_cc_no(&mut self, value: u8) {
        self.in_cc_no = value;
    }

    fn set_out_cc_no(&mut self, value: u8) { self.out_cc_no = value; }
}

impl MidiReceiver for MidiProcessor {
    fn on_message(&mut self, raw_message: &[u8]) {
        let out_message = self.get_message_to_send(raw_message);
        if let Some(out_message) = out_message {
            let mut midi_sender= self.midi_sender.lock().unwrap();
            midi_sender.send_message(out_message);
        }
    }
}
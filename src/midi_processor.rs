use std::sync::{Arc, Mutex};
use midly::{MidiMessage, live::LiveEvent, num::u7};
use simonor::{MidiReceiver, MidiSender, SharedMidiSender};
use crate::{CcNoRangeDefaults, CcNoRangeSetters};

pub trait IMidiProcessor: CcNoRangeSetters + MidiReceiver {}

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
    
    fn get_message_to_send(&mut self, raw_message: &[u8]) -> Option<Vec<u8>> {
        let event = LiveEvent::parse(raw_message).unwrap();
        if let LiveEvent::Midi { channel, message } = event {
            let channel1 = u8::from(channel) + 1; // 1-based channel
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
                                let new_message =
                                    MidiSender::create_control_change(
                                        channel1, self.out_cc_no, u8::from(value));
                                return Some(new_message);
                            }
                            Some(raw_message.to_vec())
                        }
                        _ => {
                            Some(raw_message.to_vec())
                        }
                    }
                }
                MidiMessage::NoteOn { key, vel } => {
                    let new_message =
                        MidiSender::create_note_on(
                            channel1, self.transpose_key(key), u8::from(vel));
                    Some(new_message)
                }
                MidiMessage::NoteOff { key, vel } => {
                    let new_message =
                        MidiSender::create_note_off(
                            channel1, self.transpose_key(key), u8::from(vel));
                    Some(new_message)
                }
                MidiMessage::Aftertouch { key, vel } => {
                    let new_message =
                        MidiSender::create_note_aftertouch(
                            channel1, self.transpose_key(key), u8::from(vel));
                    Some(new_message)
                }
                _ => {
                    Some(raw_message.to_vec())
                }
            }
        } else {
            // A System Common message, such as System Exclusive,
            // or a one-byte System Realtime message.
            Some(raw_message.to_vec())
        }
    }

    fn transpose_key(&self, in_key: u7) -> u8 {
        let mut transposed: i16 =
            i16::from(u8::from(in_key)) + i16::from(self.octave_shift * 12);
        while transposed < 0 {
            transposed += 12;
        }
        while transposed > 127 {
            transposed -= 12;
        }
        transposed as u8
    }
}

impl IMidiProcessor for MidiProcessor {}

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
            midi_sender.send_message(&out_message);
        }
    }
}

impl MidiReceiver for Box<dyn IMidiProcessor> {
    fn on_message(&mut self, raw_message: &[u8]) {
        (**self).on_message(raw_message);
    }
}

pub type SharedMidiProcessor = Arc<Mutex<Box<dyn IMidiProcessor>>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, MutexGuard};
    use googletest::assert_that;
    use googletest::matchers::eq;
    use simonor::MidiSender;
    use simonor::testing::{mock_midi_sender, MockMidiSender};

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[googletest::gtest]
    fn test_down() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        // No shift
        on_note_on(&mut midi_processor, 60, 61);
        assert_that!(mock_midi_sender().note_on_count, eq(1));
        assert_that!(mock_midi_sender().note_on_key, eq(60));
        assert_that!(mock_midi_sender().note_on_vel, eq(61));
        on_note_off(&mut midi_processor, 60, 71);
        assert_that!(mock_midi_sender().note_off_count, eq(1));
        assert_that!(mock_midi_sender().note_off_key, eq(60));
        assert_that!(mock_midi_sender().note_off_vel, eq(71));
        on_note_aftertouch(&mut midi_processor, 60, 81);
        assert_that!(mock_midi_sender().note_aftertouch_count, eq(1));
        assert_that!(mock_midi_sender().note_aftertouch_key, eq(60));
        assert_that!(mock_midi_sender().note_aftertouch_vel, eq(81));
        // Down 1 octave
        on_down(&mut midi_processor);
        assert_that!(mock_midi_sender().control_change_count, eq(0));
        on_note_on(&mut midi_processor, 60, 62);
        assert_that!(mock_midi_sender().note_on_count, eq(2));
        assert_that!(mock_midi_sender().note_on_key, eq(48));
        assert_that!(mock_midi_sender().note_on_vel, eq(62));
        on_note_off(&mut midi_processor, 60, 72);
        assert_that!(mock_midi_sender().note_off_count, eq(2));
        assert_that!(mock_midi_sender().note_off_key, eq(48));
        assert_that!(mock_midi_sender().note_off_vel, eq(72));
        on_note_aftertouch(&mut midi_processor, 60, 82);
        assert_that!(mock_midi_sender().note_aftertouch_count, eq(2));
        assert_that!(mock_midi_sender().note_aftertouch_key, eq(48));
        assert_that!(mock_midi_sender().note_aftertouch_vel, eq(82));
        // Down 2 octaves
        on_down(&mut midi_processor);
        on_note_on(&mut midi_processor, 60, 63);
        assert_that!(mock_midi_sender().note_on_count, eq(3));
        assert_that!(mock_midi_sender().note_on_key, eq(36));
        assert_that!(mock_midi_sender().note_on_vel, eq(63));
        // Down 3 octaves
        on_down(&mut midi_processor);
        on_note_on(&mut midi_processor, 60, 64);
        assert_that!(mock_midi_sender().note_on_count, eq(4));
        assert_that!(mock_midi_sender().note_on_key, eq(24));
        assert_that!(mock_midi_sender().note_on_vel, eq(64));
        // Down too many octaves
        on_down(&mut midi_processor);
        on_note_on(&mut midi_processor, 60, 65);
        assert_that!(mock_midi_sender().note_on_count, eq(5));
        assert_that!(mock_midi_sender().note_on_key, eq(24));
        assert_that!(mock_midi_sender().note_on_vel, eq(65));
    }

    #[googletest::gtest]
    fn test_up() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        // No shift
        on_note_on(&mut midi_processor, 60, 61);
        assert_that!(mock_midi_sender().note_on_count, eq(1));
        assert_that!(mock_midi_sender().note_on_key, eq(60));
        assert_that!(mock_midi_sender().note_on_vel, eq(61));
        on_note_off(&mut midi_processor, 60, 71);
        assert_that!(mock_midi_sender().note_off_count, eq(1));
        assert_that!(mock_midi_sender().note_off_key, eq(60));
        assert_that!(mock_midi_sender().note_off_vel, eq(71));
        on_note_aftertouch(&mut midi_processor, 60, 81);
        assert_that!(mock_midi_sender().note_aftertouch_count, eq(1));
        assert_that!(mock_midi_sender().note_aftertouch_key, eq(60));
        assert_that!(mock_midi_sender().note_aftertouch_vel, eq(81));
        // Up 1 octave
        on_up(&mut midi_processor);
        assert_that!(mock_midi_sender().control_change_count, eq(0));
        on_note_on(&mut midi_processor, 60, 62);
        assert_that!(mock_midi_sender().note_on_count, eq(2));
        assert_that!(mock_midi_sender().note_on_key, eq(72));
        assert_that!(mock_midi_sender().note_on_vel, eq(62));
        on_note_off(&mut midi_processor, 60, 72);
        assert_that!(mock_midi_sender().note_off_count, eq(2));
        assert_that!(mock_midi_sender().note_off_key, eq(72));
        assert_that!(mock_midi_sender().note_off_vel, eq(72));
        on_note_aftertouch(&mut midi_processor, 60, 82);
        assert_that!(mock_midi_sender().note_aftertouch_count, eq(2));
        assert_that!(mock_midi_sender().note_aftertouch_key, eq(72));
        assert_that!(mock_midi_sender().note_aftertouch_vel, eq(82));
        // Up 2 octaves
        on_up(&mut midi_processor);
        on_note_on(&mut midi_processor, 60, 63);
        assert_that!(mock_midi_sender().note_on_count, eq(3));
        assert_that!(mock_midi_sender().note_on_key, eq(84));
        assert_that!(mock_midi_sender().note_on_vel, eq(63));
        // Up 3 octaves
        on_up(&mut midi_processor);
        on_note_on(&mut midi_processor, 60, 64);
        assert_that!(mock_midi_sender().note_on_count, eq(4));
        assert_that!(mock_midi_sender().note_on_key, eq(96));
        assert_that!(mock_midi_sender().note_on_vel, eq(64));
        // Up too many octaves
        on_up(&mut midi_processor);
        on_note_on(&mut midi_processor, 60, 65);
        assert_that!(mock_midi_sender().note_on_count, eq(5));
        assert_that!(mock_midi_sender().note_on_key, eq(96));
        assert_that!(mock_midi_sender().note_on_vel, eq(65));
    }

    #[googletest::gtest]
    fn test_down_too_low() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        on_down(&mut midi_processor);
        on_note_on(&mut midi_processor, 11, 62);
        assert_that!(mock_midi_sender().note_on_count, eq(1));
        assert_that!(mock_midi_sender().note_on_key, eq(11));
        assert_that!(mock_midi_sender().note_on_vel, eq(62));
    }

    #[googletest::gtest]
    fn test_up_too_high() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        on_up(&mut midi_processor);
        on_note_on(&mut midi_processor, 116, 62);
        assert_that!(mock_midi_sender().note_on_count, eq(1));
        assert_that!(mock_midi_sender().note_on_key, eq(116));
        assert_that!(mock_midi_sender().note_on_vel, eq(62));
    }

    #[googletest::gtest]
    fn test_in() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        on_in(&mut midi_processor, 108);
        assert_that!(mock_midi_sender().control_change_count, eq(1));
        assert_that!(mock_midi_sender().control_change_cc_no, eq(midi_processor.out_cc_no));
        assert_that!(mock_midi_sender().control_change_value, eq(108));
    }

    #[googletest::gtest]
    fn test_out_is_up() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        midi_processor.up_cc_no = midi_processor.out_cc_no;
        on_in(&mut midi_processor, 127);
        on_note_on(&mut midi_processor, 60, 61);
        assert_that!(mock_midi_sender().note_on_key, eq(72));
    }

    #[googletest::gtest]
    fn test_other_control_change() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        on_control_change(&mut midi_processor, 4, 108);
        assert_that!(mock_midi_sender().control_change_count, eq(1));
        assert_that!(mock_midi_sender().control_change_cc_no, eq(4));
        assert_that!(mock_midi_sender().control_change_value, eq(108));
    }

    #[googletest::gtest]
    fn test_other_midi() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        midi_processor.on_message(&*MidiSender::create_channel_aftertouch(
            1, 17));
        assert_that!(mock_midi_sender().other_midi_count, eq(1));
    }

    #[googletest::gtest]
    fn test_non_midi() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        midi_processor.on_message(&[0xF0, 1]); // System exclusive
        assert_that!(mock_midi_sender().non_midi_count, eq(1));
    }

    fn on_down(midi_processor: &mut MidiProcessor) {
        on_control_change(midi_processor, midi_processor.down_cc_no, 127);
    }

    fn on_up(midi_processor: &mut MidiProcessor) {
        on_control_change(midi_processor, midi_processor.up_cc_no, 127);
    }

    fn on_in(midi_processor: &mut MidiProcessor, value: u8) {
        on_control_change(midi_processor, midi_processor.in_cc_no, value);
    }

    fn on_control_change(midi_processor: &mut MidiProcessor, cc_no: u8, value: u8) {
        midi_processor.on_message(&*MidiSender::create_control_change(
            1, cc_no, value));
    }

    fn on_note_on(midi_processor: &mut MidiProcessor, key: u8, velocity: u8) {
        midi_processor.on_message(&*MidiSender::create_note_on(
            1, key, velocity));
    }

    fn on_note_off(midi_processor: &mut MidiProcessor, key: u8, velocity: u8) {
        midi_processor.on_message(&*MidiSender::create_note_off(
            1, key, velocity));
    }

    fn on_note_aftertouch(midi_processor: &mut MidiProcessor, key: u8, pressure: u8) {
        midi_processor.on_message(&*MidiSender::create_note_aftertouch(
            1, key, pressure));
    }

    /// To avoid races on static data, hold the returned guard in each test to ensure sequential
    /// execution of tests.
    fn test_mutex_guard() -> MutexGuard<'static, ()> {
        TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
    }
}
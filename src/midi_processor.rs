use std::sync::{Arc, Mutex, Weak};
// use log::{debug};
use log::{trace};
use midly::{MidiMessage, live::LiveEvent, num::u7};
use simonor::{MidiReceiver, MidiSender, SharedMidiSender};
use crate::{CcNoRangeDefaults, CcNoRangeSetters, OctaveShiftListener};

#[derive(Copy, Clone)]
struct TransposedNote {
    channel1: u8, // 1-based channel
    original_key: u8,
    transposed_key: u8,
}

impl TransposedNote {
    pub fn new(channel1: u8, original_key: u8, transposed_key: u8) -> Self {
        Self {
            channel1,
            original_key,
            transposed_key,
        }
    }
}

pub trait IMidiProcessor: CcNoRangeSetters + MidiReceiver {
    /// Records the (weak) semantic listener the processor raises events to. Weak to avoid a
    /// reference cycle, mirroring the `Presenter`'s `presenter_weak`. Set by `Presenter::init`.
    fn set_listener<'a>(&self, listener: Weak<dyn OctaveShiftListener>);
}

pub struct MidiProcessor {
    /// The semantic listener (the `Presenter`). Weak to avoid a reference cycle; set by
    /// `Presenter::init`.
    listener: Mutex<Option<Weak<dyn OctaveShiftListener>>>,
    down_cc_no: u8,
    up_cc_no: u8,
    in_cc_no: u8,
    out_cc_no: u8,
    octave_shift: i8,
    midi_sender: SharedMidiSender,
    transposed_notes: Arc<Mutex<Vec<TransposedNote>>>,
}

impl MidiProcessor {
    pub fn new(midi_sender: SharedMidiSender) -> Self {
        Self {
            listener: Mutex::new(None),
            down_cc_no: CcNoRangeDefaults::down_cc_no(),
            up_cc_no: CcNoRangeDefaults::up_cc_no(),
            in_cc_no: CcNoRangeDefaults::in_cc_no(),
            out_cc_no: CcNoRangeDefaults::out_cc_no(),
            octave_shift: 0,
            midi_sender,
            transposed_notes: Arc::new(Mutex::new(vec![])),
        }
    }

    fn find_and_remove_transposed_note(
            &self, channel1: u8, original_key: u8) -> Option<TransposedNote> {
        let mut transposed_notes = self.transposed_notes.lock().unwrap();
        transposed_notes.iter()
            .position(|note|
                note.channel1 == channel1 && note.original_key == original_key)
            .map(|index| transposed_notes.remove(index))
    }

    fn find_transposed_note(
        &self, channel1: u8, original_key: u8) -> Option<TransposedNote> {
        let transposed_notes = self.transposed_notes.lock().unwrap();
        transposed_notes.iter()
            .find(|note|
                note.channel1 == channel1 && note.original_key == original_key)
            .copied()
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
                                    self.on_octave_shift_changed();
                                }
                            }
                            None
                        }
                        controller if controller == self.up_cc_no => {
                            if value == 127 {
                                if self.octave_shift < 3 {
                                    self.octave_shift += 1;
                                    self.on_octave_shift_changed();
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
                    let transposed_key_opt = self.transpose_key(key);
                    // Don't play note if transposition would be out of range.
                    if transposed_key_opt.is_none() {
                        return None;
                    }
                    let transposed_key = transposed_key_opt.unwrap();
                    trace!("NoteOn: `from key {} to key {}", key, transposed_key);
                    let transposed_note =
                        TransposedNote::new(channel1, u8::from(key), transposed_key);
                    self.transposed_notes.lock().unwrap().push(transposed_note);
                    let new_message =
                        MidiSender::create_note_on(
                            channel1, transposed_key, u8::from(vel));
                    Some(new_message)
                }
                MidiMessage::NoteOff { key, vel } => {
                    // The octave shift may have changed between the NoteOn and NoteOff messages.
                    // Or the transposition may have been out of range, in which case the note
                    // won't be playing.
                    // So,if the note is playing,
                    // we need to set the NoteOff's key to the NoteOn's transposed key,
                    // which is not necessarily the same as would be calculated from the current
                    // octave shift.
                    if let Some(transposed_note) =
                            self.find_and_remove_transposed_note(channel1, u8::from(key)) {
                        trace!("NoteOff: `from key {} to key {}", key, transposed_note.transposed_key);
                        let new_message =
                            MidiSender::create_note_off(
                                channel1, transposed_note.transposed_key, u8::from(vel));
                        Some(new_message)
                    } else { // The note isn't playing.
                        None
                    }
                }
                MidiMessage::Aftertouch { key, vel } => {
                    // The octave shift may have changed between the NoteOn and NoteAftertouch
                    // messages.
                    // Or the transposition may have been out of range, in which case the note
                    // won't be playing.
                    // So,if the note is playing,
                    // we need to set the NoteOff's key to the NoteOn's transposed key,
                    // So we need to set the NoteAftertouch's key to the NoteOn's transposed key,
                    // which is not necessarily the same as would be calculated from the current
                    // octave shift.
                    if let Some(transposed_note) =
                        self.find_transposed_note(channel1, u8::from(key)) {
                        trace!("NoteAftertouch: `from key {} to key {}", key, transposed_note.transposed_key);
                        let new_message =
                            MidiSender::create_note_aftertouch(
                                channel1, transposed_note.transposed_key, u8::from(vel));
                        Some(new_message)
                    } else { // The note isn't playing.
                        None
                    }
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

    /// The semantic listener, if it is still alive..
    fn listener(&self) -> Option<Arc<dyn OctaveShiftListener>> {
        self.listener.lock().unwrap().as_ref().and_then(
            |weak| weak.upgrade())
    }

    fn on_octave_shift_changed(&self) {
        // Prefix the octave shift value with a plus or minus sign unless 0.
        let octave_shift_text =
            if self.octave_shift == 0 {
                "0".to_string()
            } else {
                format!("{}{}",
                        if self.octave_shift > 0 {
                            "+"
                        } else {
                            "-"
                        },
                        self.octave_shift.abs())
            };
        let info_message = format!("Octave shift: {}", octave_shift_text);
        trace!("on_octave_shift_changed: {}", info_message);
        if let Some(listener) = self.listener() {
            rayon::spawn(move || listener.on_octave_shift_changed(info_message));
        }
    }

    /// Transpose a key by the octave shift value, returning the transposed key or,
    /// if that would be out of range, None.
    fn transpose_key(&self, in_key: u7) -> Option<u8>  {
        let transposed: i16 =
            i16::from(u8::from(in_key)) + i16::from(self.octave_shift * 12);
        if transposed < 0 || transposed > 127 {
            return None;
        }
        Some(transposed as u8)
    }
}

impl IMidiProcessor for MidiProcessor {
    fn set_listener(&self, listener: Weak<dyn OctaveShiftListener>) {
        *self.listener.lock().unwrap() = Some(listener);
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
    use std::sync::{MutexGuard};
    use googletest::assert_that;
    use googletest::matchers::eq;
    use simonor::MidiSender;
    use simonor::testing::{mock_midi_sender, MockMidiSender};

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    static INFO_MESSAGE: Mutex<String> = Mutex::new(String::new());

    pub fn info_message() -> String {
        INFO_MESSAGE.lock().unwrap().clone()
    }

    pub fn reset_info_message() {
        INFO_MESSAGE.lock().unwrap().clear();
    }

    pub fn wait_for_info_message() -> String {
        use std::time::{Duration, Instant};
        let start = Instant::now();
        loop {
            let message = info_message();
            if !message.is_empty() || start.elapsed() > Duration::from_secs(5) {
                return message;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    struct MockListener {}

    impl MockListener {
        fn new() -> Self { MockListener {} }
    }

    impl OctaveShiftListener for MockListener {
        fn on_octave_shift_changed(&self, info_message: String) {
            let mut info_message_guard = INFO_MESSAGE.lock().unwrap();
            *info_message_guard = info_message;
        }
    }

    impl OctaveShiftListener for Mutex<MockListener> {
        fn on_octave_shift_changed(&self, info_message: String) {
            self.lock().unwrap().on_octave_shift_changed(info_message);
        }
    }

    #[googletest::gtest]
    fn test_octave_shift_changed_message() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        let mock_listener: Arc<dyn OctaveShiftListener> = Arc::new(MockListener::new());
        midi_processor.set_listener(Arc::downgrade(&mock_listener));
        reset_info_message();
        on_down(&mut midi_processor); // Down 1 octave
        assert_that!(wait_for_info_message(), eq("Octave shift: -1"));
        reset_info_message();
        on_up(&mut midi_processor); // Back to no shift
        assert_that!(wait_for_info_message(), eq("Octave shift: 0"));
        reset_info_message();
        on_up(&mut midi_processor); // Up 1 octave
        assert_that!(wait_for_info_message(), eq("Octave shift: +1"));
    }

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
        on_note_aftertouch(&mut midi_processor, 60, 81);
        assert_that!(mock_midi_sender().note_aftertouch_count, eq(1));
        assert_that!(mock_midi_sender().note_aftertouch_key, eq(60));
        assert_that!(mock_midi_sender().note_aftertouch_vel, eq(81));
        on_note_off(&mut midi_processor, 60, 71);
        assert_that!(mock_midi_sender().note_off_count, eq(1));
        assert_that!(mock_midi_sender().note_off_key, eq(60));
        assert_that!(mock_midi_sender().note_off_vel, eq(71));
        // Down 1 octave
        on_down(&mut midi_processor);
        assert_that!(mock_midi_sender().control_change_count, eq(0));
        on_note_on(&mut midi_processor, 60, 62);
        assert_that!(mock_midi_sender().note_on_count, eq(2));
        assert_that!(mock_midi_sender().note_on_key, eq(48));
        assert_that!(mock_midi_sender().note_on_vel, eq(62));
        on_note_aftertouch(&mut midi_processor, 60, 82);
        assert_that!(mock_midi_sender().note_aftertouch_count, eq(2));
        assert_that!(mock_midi_sender().note_aftertouch_key, eq(48));
        assert_that!(mock_midi_sender().note_aftertouch_vel, eq(82));
        on_note_off(&mut midi_processor, 60, 72);
        assert_that!(mock_midi_sender().note_off_count, eq(2));
        assert_that!(mock_midi_sender().note_off_key, eq(48));
        assert_that!(mock_midi_sender().note_off_vel, eq(72));
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
        on_note_aftertouch(&mut midi_processor, 60, 81);
        assert_that!(mock_midi_sender().note_aftertouch_count, eq(1));
        assert_that!(mock_midi_sender().note_aftertouch_key, eq(60));
        assert_that!(mock_midi_sender().note_aftertouch_vel, eq(81));
        on_note_off(&mut midi_processor, 60, 71);
        assert_that!(mock_midi_sender().note_off_count, eq(1));
        assert_that!(mock_midi_sender().note_off_key, eq(60));
        assert_that!(mock_midi_sender().note_off_vel, eq(71));
        // Up 1 octave
        on_up(&mut midi_processor);
        assert_that!(mock_midi_sender().control_change_count, eq(0));
        on_note_on(&mut midi_processor, 60, 62);
        assert_that!(mock_midi_sender().note_on_count, eq(2));
        assert_that!(mock_midi_sender().note_on_key, eq(72));
        assert_that!(mock_midi_sender().note_on_vel, eq(62));
        on_note_aftertouch(&mut midi_processor, 60, 82);
        assert_that!(mock_midi_sender().note_aftertouch_count, eq(2));
        assert_that!(mock_midi_sender().note_aftertouch_key, eq(72));
        assert_that!(mock_midi_sender().note_aftertouch_vel, eq(82));
        on_note_off(&mut midi_processor, 60, 72);
        assert_that!(mock_midi_sender().note_off_count, eq(2));
        assert_that!(mock_midi_sender().note_off_key, eq(72));
        assert_that!(mock_midi_sender().note_off_vel, eq(72));
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
    fn test_octave_shift_between_note_on_and_note_off() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        // Up 1 octave
        on_up(&mut midi_processor);
        on_note_on(&mut midi_processor, 60, 61);
        assert_that!(mock_midi_sender().note_on_key, eq(72));
        assert_that!(mock_midi_sender().note_on_vel, eq(61));
        // Up 2 octaves
        on_up(&mut midi_processor);
        on_note_off(&mut midi_processor, 60, 62);
        // Confirm that the NoteOff's key has been transposed to the NoteOn's transposed key,
        // which is not the same as would be calculated from the new octave shift.
        assert_that!(mock_midi_sender().note_off_key, eq(72));
        assert_that!(mock_midi_sender().note_off_vel, eq(62));
    }

    #[googletest::gtest]
    fn test_down_too_low() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        on_down(&mut midi_processor);
        on_note_on(&mut midi_processor, 11, 62);
        assert_that!(mock_midi_sender().note_on_count, eq(0));
    }

    #[googletest::gtest]
    fn test_up_too_high() {
        let _guard = test_mutex_guard();
        let mut midi_processor =
            MidiProcessor::new(Arc::new(Mutex::new(MockMidiSender::new())));
        on_up(&mut midi_processor);
        on_note_on(&mut midi_processor, 116, 62);
        assert_that!(mock_midi_sender().note_on_count, eq(0));
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
# Octave Up-Down for MIDI Pedals

Octave Up-Down allows a pair of sustain pedals or momentary footswitches to be used instead of the octave shift buttons on a MIDI keyboard to transpose notes and note aftertouch /polyphonic aftertouch / pressure up or down by multiple octaves. A bonus feature allows the control change number of a MIDI controller to be changed: the controller may be one of the pedals used for octave transposition.

<img src="docs\images\Octave Up-Down.png" alt="Octave Up-Down" style="zoom: 80%;" />

## MIDI Connections

The MIDI signals from the sustain pedals and the keyboard need to be merged to connect to Octave-Up Down's **MIDI Input**.  This is easy with an Arturia KeyLab keyboard: the the two pedals/footswitches to be used for octave transposition can be connected to two of the keyboard's three pedal inputs, leaving the third input for an expression pedal.  Another option is to connect the pedals/footswitches to a MIDI pedal converter, such as a  [DOREMIDI MPC-10](https://www.doremidi.cn/h-pd-48.html) or [MPC20](https://www.doremidi.cn/h-pd-71.html), and connect the pedal converter's MIDI out to the keyboard's MIDI in.  For intuitive playing, use the left pedal for Down and the right pedal for Up.

Connect Octave-Up Down's **MIDI Output** to a sound engine, such as a synthesizer, via one of these:

- A MIDI loopback endpoint, provided in Windows by Microsoft's recently introduced Windows MIDI Services.

- An IAC Driver Bus (macOS).

- A third-party virtual MIDI port, such as are provided by [Bome](https://www.bome.com/).

- A USB MIDI router, such as [IConnectivity's mioXL and mioXM](https://www.iconnectivity.com/midi-interfaces-1).

## How the Octave Shift Works

Octave Up-Down emulates the usual behaviour or a MIDI keyboard's octave buttons. Pressing the down (left) pedal once shifts the note transposition down by one octave.  Pressing the up (right) pedal shifts the note transposition up by one octave.  Transposition can can be shifted a maximum of three octaves above or three octaves below the default.  The keys of the following MIDI message types are transposed:  Note On, Note Aftertouch (a.k.a. Polyphonic Aftertouch) and Note Off.  If transposition would result in a note outside the MIDI range 0 to 127, a transposed note is not output, resulting in silence.

## Changing a Control Change Number

To change a control change number, specify **In CC No** and **Out CC NO**.  If they are equal, no change will be attempted.  If the CC number changed is that of the Down or Up controller, **Out CC NO** must equal whichever of **Down CC No** or **Up CC No** applies.

## Supported Operating Systems

Windows 64-bit and macOS Universal (Apple Silicon + Intel).

## macOS First-Time Launch Instructions

In the DMG, drag **Octave Up-Down** into /Applications. Then take one of the following approaches.

#### Approach A

1. Open the app. When a security prompt headed **'“Octave Up-Down” Not Opened'**  appears, click **Done**. *Do not click Move to Bin*.
2. Go to **System Settings** → **Privacy & Security**, scroll down to **Security**, and click **Open Anyway**.
3. On the confirmation prompt, click **Open Anyway**. *(This is only required on the very first launch).*

#### Approach B

Run this in a terminal:

```bash
xattr -cr "/Applications/Octave Up-Down.app"
```

Once executed, the application will open directly upon double-clicking.

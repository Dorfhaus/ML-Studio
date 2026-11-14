# ML Studio User Guide

## Start here
Overview: Install, set up audio, and make your first song.

### Make your first song
![workspace | Main workspace with the Channel Rack, Playlist and Piano Roll](assets/guide/workspace.jpg)
Keywords: beat beginner first project getting started
Related: Understand the main windows | Set up an audio device | Program steps and patterns | Export a finished song, stems or MIDI
Guide: rack-steps
1. Use File > New to start with your default template. Set the template in General settings and choose an output device in Options > Audio settings.
2. Add an instrument with the Add menu or the plugin picker. Open the Channel Rack and click steps on a drum channel to make a beat.
3. Right-click an instrument's row and open Piano Roll to draw notes. Play in Pattern mode to hear the current pattern.
4. Open the Playlist, choose the pattern and click a track to place it. Switch to Song mode to hear the arrangement.
5. Save the project, then use File > Export audio to create an audio file.

### Understand the main windows
Keywords: workspace browser mixer channel rack piano roll playlist arrangement
![workspace | Channel Rack, Playlist and Piano Roll in the workspace](assets/guide/workspace.jpg)
Related: Show a window or restore your workspace | Build an arrangement | Set levels, pan and routing
The Channel Rack holds instruments, samples and step patterns. Piano Roll edits notes for one channel in the current pattern. The Playlist arranges patterns, recordings and automation over time.
The Mixer controls audio level, pan, effects and routing. The Browser finds samples, projects and presets. Use the toolbar shortcuts or the View menu to show these windows.

### Set tempo and play the right thing
Keywords: bpm metronome transport playback loop song pattern space
![workspace | Transport controls along the top of the workspace](assets/guide/workspace.jpg)
Guide: transport
Choose PAT to hear the current pattern or SONG to hear the Playlist. Press Space to start or stop, or use the toolbar transport.
Adjust BPM in the toolbar. Enable the metronome to hear the beat and click in the Playlist ruler to set the playhead.

### Browse or search the User Guide
Keywords: offline help manual collapsible sections topics find
Open Help > User Guide. Choose a chapter from the home page or Contents sidebar, then select an article. Use Back to return to the previous article or list position, Home to return to the chapter index, and Related articles or Previous / Next article to keep reading.
Press Ctrl+K or Ctrl+F to focus search, then enter a feature, shortcut or task. Enter opens the first match. Click a picture to enlarge it and Escape to return. Where available, Show me where points to the matching control in the workspace. Return to Guide restores the article.

Use Detach at the top of the guide to open a resizable native window. Drag its title bar to another monitor and keep editing the project in the main window. Use Dock to return to the app. Close hides the guide, and Help > User Guide reopens it in its last mode. Workspace Visibility > Help can hide the Detach button.

### Install and open ML Studio
Keywords: download install Windows macOS Apple silicon Intel Linux update .lmn first launch
Get the installer or archive for your operating system and processor from the official ML Studio downloads page, https://michaellanemusic.de/ml-studio/. Windows uses the installer. On macOS, choose Apple silicon or Intel, then place the application in a writable Applications folder. On Linux x86-64, extract the full archive into a writable folder and launch ML Studio from it.
1. Start ML Studio and open Options > Audio settings. Pick your interface or built-in output.
2. Use File > New from template to choose a starting workspace. Lemon Starter is the supplied starting template.
3. Use Help > About to identify your build. Help > Change Log lists the installed release history. When a signed update is available, the update control downloads it and offers a restart.
Related: Make your first song | Set up an audio device | Report a problem and check for updates

### Find a control that is missing or greyed out
Keywords: missing hidden disabled grayed greyed unavailable menu option setting button does not show find control
Open Settings > Visibility and search for the label. Enable its visibility if it is hidden, then return to the original menu or Settings page. Some controls appear in an **Advanced** tab or a collapsed settings group. If a menu entry is missing, also check Settings > Menus for that menu's layout.
If the control is visible but grey, check what is selected and whether the action needs a saved project, stopped playback, an unlocked track or an installed plugin. A feature can also depend on the platform or audio driver. The toolbar hint area can describe supported controls as you move the pointer over them.
Related: Show or hide interface features | How menus work | Restore a shortcut that stopped working

### Read the toolbar and project time
Keywords: toolbar hint time display song position varispeed output monitor cpu pattern snap recording
The hint area describes the control under the pointer. PAT and SONG choose what transport plays. Use the pattern selector to choose the current pattern and the Global snap selector to set the shared editing grid. Editors with an independent snap setting can follow the global choice or override it.
The main level and master mixer fader control different stages of the output path. Check both if the signal is low. The main pitch or varispeed and the tempo controls affect playback. For a local clip timing change, use that clip's settings or Audio Timing.
The output monitor can show a triggered stereo waveform, peaks, processing load, memory and underruns. Options > CPU meter switches among Compact, Classic FL-style, and Lemon live monitor. Use the counters while reproducing an audible problem.
Guide: transport
Related: Set tempo and play the right thing | Fix delay, crackles or no sound

## Audio & recording
Overview: Choose devices and record audio, MIDI and automation.

### Set up an audio device
Keywords: sound output speakers headphones interface asio wasapi coreaudio buffer latency
![audio | Audio settings with device and latency controls](assets/guide/audio.jpg)
Related: Record a microphone or instrument | Fix delay, crackles or no sound
Guide: audio-device
Open Options > Audio settings. Choose an output device and verify that the device's sample rate and channels match your interface settings.
Start with a buffer that plays reliably. Lower it if monitoring feels delayed, or raise it if playback crackles or the underrun count rises. On Windows, select your interface's ASIO driver if available.

### Record a microphone or instrument
Keywords: record vocals guitar interface input arm monitor take
Related: Prevent feedback and choose a recording input | Comp and organize takes
Guide: record
1. Select an empty Mixer insert and choose your interface input in its input selector.
2. Check that the meter moves and set a safe input level on the interface. Arm disk recording on the insert.
3. Enable recording and choose audio recording if prompted. Start playback, perform the take, then stop and check the recorded clip.
Use headphones when monitoring a microphone. If the sound is doubled, check whether both interface direct monitoring and software monitoring are active.

### Prevent feedback and choose a recording input
Keywords: record dry mono stereo left right input monitoring bleed
Guide: mixer-input
Use a mono input for a single microphone or instrument, and a stereo pair for a stereo source. Verify the input selector on the Mixer track before arming it.
Listen on headphones and disable any unintended second monitoring path. Record a short test, then play it back to check the level, channel balance and timing.

### Record notes from a MIDI keyboard
Keywords: midi controller record piano keyboard velocity
Guide: midi
Enable the device in Options > MIDI settings. Select the instrument channel, test a few notes, then enable recording and select note recording if prompted.
Start playback in the desired mode and perform. Open the pattern's Piano Roll to correct timing and velocity.

### Fix delay, crackles or no sound
Keywords: latency underruns cpu stutter dropout silence device troubleshooting
Guide: audio-buffer
If nothing plays, check the output device, master level, muted tracks, song/pattern mode and routing to Master. If only one channel is silent, check its sample or plugin and its Mixer destination.
For dropouts, raise the buffer and temporarily bypass expensive effects to isolate the cause. Freeze or bounce finished parts if needed.
Playback keeps one full device buffer in reserve, with a 3 ms minimum, to tolerate closely spaced audio callbacks. Small buffers remain best for live playing and recording. Larger buffers provide more protection but also increase the time before edits or live notes are heard.
Plugin delay compensation aligns paths using delays reported by plugins. Input monitoring still has hardware and processing latency, so use a small stable buffer or direct monitoring when recording.

### Set up a keyboard or mixer controller
Keywords: MIDI controller Komplete Kontrol KeyLab Arturia Behringer X-Touch nanoKONTROL Akai Novation Launchkey M-Audio Nektar Mackie MCU
![midi | MIDI settings with device and controller configuration](assets/guide/midi.jpg)
Guide: midi
Open Settings > MIDI, select an input device, enable it and choose its Controller type. This choice is saved separately for each device in your profile.
For musical keys and pads, choose Generic MIDI / MIDI Learn. Use the input role to separate an instrument port from a dedicated DAW controls port. Use Instruments + controls when a keyboard has one shared port. Right-click a software knob, choose Link to controller, then move the hardware control. For an endless encoder, choose the Relative mode that matches its MIDI format.
Under DAW MIDI Learn, select an action and press a hardware button on the selected input. Single press mode suits CC buttons that send no release. Save Learn preset exports the actions and button behavior to a file. Load Learn preset applies that file to the selected input and replaces its learned actions.
For a controller with Mackie Control Universal support, set its hardware mode to MCU, select its dedicated DAW input in ML Studio, and choose Mackie Control or the matching named MCU profile. Profiles include Kontrol, KeyLab, X-Touch, Oxygen Pro, nanoKONTROL Studio, SSL UF1/UF8 and iCON QCon. Use the hardware's matching Mackie mode. Select the dedicated DAW output in the output list, then click Pair selected output. Hardware with compatible motor faders, button LEDs, pan rings or track-name displays receives feedback over that output.
For Launchkey MK4, select its native profile on the DAW input and pair the DAW output to enter DAW mode. Its DAW pads select and mute Mixer inserts, and its mixer encoders and available faders control pan and level. Keep the instrument port available for playing keys. The nanoKONTROL2 factory profile uses CC mode. For Akai and other controllers configured to transmit MIDI Machine Control, choose the MMC profile to receive transport commands alongside notes.
If notes work but transport does not, turn on the MIDI Monitor and check whether pressing a button generates a message on the selected DAW port. If an input shows in use, close the other application using that MIDI port. Use Refresh device list after connecting hardware.
Hide the setup controls under Settings > General > Workspace Visibility > Input customization > MIDI controller setup. Saved modes and mappings continue to operate when the controls are hidden.

### Use MIDI, typing keyboard and step entry
Keywords: step edit typing keyboard controllers performance
Guide: typing
Enable Typing keyboard to piano in the toolbar or press Ctrl+T. ML Studio remembers the switch in your profile when you quit and reopen. Notes go to the selected instrument. Use numpad + and - to shift the keyboard by octaves. Right-click the typing-keyboard button and choose Octave to set the base octave directly, including on keyboards without a numpad. The octave is saved per profile; 4 is the default. Octave changes apply to new notes, so held notes still release at their original pitch. Settings > Visibility > Input customization can hide the octave menu without changing the setting.
Right-click the toolbar button to choose Key and Scale or Follow Piano Roll typing snap. For a selected scale, the letter rows play successive scale notes and each row starts one octave above the previous row. On a German QWERTZ keyboard, the bottom row runs from Y through minus and the home row from A through Ä. The corresponding QWERTY ranges are Z through slash and A through apostrophe. Choose Chromatic for the staggered piano-key layout with black keys on the row above.
Turn typing off to use the same unmodified keys for editing shortcuts. Text fields take text normally and modified keyboard shortcuts keep their own bindings.
Use Step Edit with playback stopped to enter notes one step at a time. Check the selected channel, current position and snap before entering notes.
Use MIDI settings to configure enabled devices and controller inputs. Save a small test project before setting up a complex controller mapping.

### Enter notes one step at a time
Keywords: step edit step recording midi chord input
Guide: step
Stop playback and enable Step Edit in the toolbar. Select an instrument and set the playhead and snap.
Play a note or chord from a MIDI or typing keyboard. Each entry advances by one snap step. Turn Step Edit off before returning to live playing.

### Choose a recording mode and lead-in
Keywords: record filter notes audio automation count in precount preroll pre-roll loop blend start on input metronome cancel
Right-click Record to check which recording types are enabled. Arm a Mixer insert with a hardware input for audio, select an instrument for notes, and enable automation recording before moving controls during a take. Make a short test take and verify where the result lands.
Right-click Recording precount to choose Off, a count-in, or pre-roll of 1, 2, 4 or 8 bars. Pre-roll plays the existing arrangement before the recording point, so you can hear your cue. The take begins at the intended point. Pre-roll clamps to the start of the song, and stopping during the lead-in leaves no take. Start on input waits for a qualifying input. Blend recorded notes adds new notes to the active material, and Loop record repeats the selected range.
Use the metronome or choose a separate Mixer track for it in Advanced audio settings. Use Tools > Cancel recording if you want to discard the current recording. Stop normally to keep the take.
Guide: record
Related: Record a microphone or instrument | Record notes from a MIDI keyboard | Comp and organize takes

### Set up inputs, monitoring and recording placement
Keywords: input monitoring latency dry wet record external direct software monitoring offset stereo interface
Select an input on a Mixer insert and watch the input meter. A mono input is appropriate for one microphone. A stereo input pair is appropriate for an interface receiving a stereo source. Arm the intended insert and check which Playlist track is linked or selected for the take.
Use one monitoring path to prevent hearing a direct and a delayed copy together. Interface direct monitoring gives a dry cue with minimal delay. DAW monitoring lets you hear the mixer route and its effects. A low stable buffer helps when monitoring through the app.
Test timing with a known transient before adjusting Recording offset under Audio > Advanced > Recording. A positive offset moves the recorded material later within the take. Trim the source or adjust clip gain for later edits.
Guide: mixer-input
Related: Prevent feedback and choose a recording input | Audio processing and recording settings | Fix audio recordings that land early or late

### Fix audio recordings that land early or late
Keywords: recording offset latency alignment early late ahead behind recording format loopback driver compensation
Open Settings > Audio > Advanced > Recording. **Recording format** sets the WAV format for new recordings. Choose 32-bit float, 24-bit PCM or 16-bit PCM before recording. Existing files keep their original format.
**Recording offset** adjusts recorded alignment in milliseconds when a timing estimate is available. It starts at +0.0 ms and ranges from −200 to +200 ms. If repeated test recordings land late, try a small negative correction. If they land early, try a small positive correction. Use the same input, driver and buffer while comparing takes. This control applies to future recordings.
Related: Set up inputs, monitoring and recording placement | Audio device and latency

### Recover a played phrase with the score log
Keywords: score logger dump last played midi notes retrospective history clear
If you played an idea without recording, select the target pattern and instrument, then open Tools > Dump score log to selected pattern. Choose a time window from the submenu. Open the Piano Roll to trim, move or quantize the resulting notes.
The score log is for played note input. Use audio recording or Tesla to capture an audio signal. Tools > Clear log clears the remembered note input.

## Channel Rack & Piano Roll
Overview: Build patterns, program notes, and shape expression.

### Add, replace and route an instrument
Keywords: channel rack generator vst synth sampler insert instrument
![rack | Channel Rack mixer assignments, pitch controls and step buttons](assets/guide/rack.jpg)
Guide: rack-route
Use Add or the plugin picker to create an instrument channel. To insert another instrument from the rack, right-click a row and choose Insert instrument.
Use the rack channel's Mixer track assignment to send its sound to an insert. Name the channel and insert so they are easy to find again.

### Program steps and patterns
Keywords: drum sequencer swing kick snare hi hat pattern
![rack | Steps appear to the right of the instrument channel](assets/guide/rack.jpg)
Guide: rack-steps
Choose or create a pattern. Click a step to turn it on or off, and play in PAT mode to audition it.
Keep variations in separate patterns so they can be arranged independently. Adjust swing to change the feel and use per-channel loop lengths where available.

### Draw, move and edit notes
Keywords: piano roll pencil brush select resize velocity note properties
![piano | Piano Roll note grid and lower event editor](assets/guide/piano.jpg)
Related: Quantize and shape notes | Edit per-note expression with MPE | Work with chord and scale tools
Guide: piano-notes
Open a channel's Piano Roll. Use Draw to add notes, drag a note to change time or pitch, and drag its edge to resize it.
Use Select to work on several notes. Right-click empty space with Draw or Brush to clear selection. Use the lower lane to edit velocity or other note values.
Double-click a note to inspect its properties. Set snap according to the grid you want before moving or resizing notes.

### Use Glide Notes and slide pitches
Keywords: glide slide portamento normal note type
Guide: piano-notes
Choose Glide Note in note properties to place a glide. Select an existing normal or glide note to pick up that placement type.
Place a glide against a sounding note on a compatible instrument and adjust its length to control the transition. Check the result on the instrument you are using, since third-party plugins can handle note slides differently.

### Zoom, navigate and place patterns
Keywords: piano roll alt wheel vertical zoom place pattern playlist toolbar
Guide: place-pattern
Hold Alt and scroll over a note to change velocity by one unit per notch (0–128). Hover a selected note to adjust the selected notes together, or an unselected note to edit just that note. Undo restores the edit. Alt + wheel over the keys or empty note grid still changes row height. Ctrl + wheel zooms time.
Choose Piano Roll menu > View > Keyboard width for Standard, Wide, Wider or Maximum keys. This changes the keyboard width, not note timing. The choice is saved per profile. Hide this menu in Settings > Visibility > Piano Roll > Keyboard width menu without changing the saved width. Use the horizontal overview to move through the pattern and the vertical controls to bring the notes into view.
Use Place Pattern to Playlist in the Piano Roll toolbar to put the current pattern at the Playlist playhead. It finds the first track with space for the clip, opens the Playlist and highlights the result.

### Quantize and shape notes
Keywords: quantize timing strum arpeggio flam chop glue legato flip randomize
Guide: piano-tools
Select the notes to process, then open the relevant command in the Piano Roll Tools menu. Quantize adjusts timing toward a grid. Strum offsets chord notes, and Arpeggiate turns chord notes into a repeating sequence.
Use Chop to split notes, Glue to join suitable notes and Legato to extend notes toward the following ones. Preview the tool, apply it, or cancel to keep the original.

### Choose between written arpeggios and live Sampler arpeggios
Keywords: arpeggiate piano roll note pattern bake print written arp sampler wrench held chord live
To write individual notes into a pattern, select the source notes and open **Arpeggiate** from the Piano Roll Tools menu. Preview the generated phrase, apply it to the selection, and edit the resulting notes in the grid. Undo restores the original phrase.
To have the Sampler generate notes during playback, use **Arpeggiator** on its wrench tab. This applies to that channel when the relevant MIDI notes play and leaves the Piano Roll notes as entered. Use this approach when you want to change the pulse, range or direction without rewriting the pattern.
Related: Quantize and shape notes | Set up Sampler arpeggios, echoes and tracking

### Edit per-note expression with MPE
Keywords: mpe pitch bend pressure slide timbre controller lanes
Guide: piano-mpe
Open MPE / Note expression in the Piano Roll menu. Enable it for the instrument and show the expression lanes. Use a compatible instrument or patch and match its pitch bend range.
Select one note to edit its expression curves. For a hardware controller, also match its zone and member channels to the instrument settings.

### Work with chord and scale tools
Keywords: chord workspace harmony key scale scale highlighting chord progressions
Guide: chords
Open Chord Workspace from the Piano Roll shortcut where shown. Choose a scale or chord and audition it in the current instrument before placing notes.
Use the Piano Roll's scale guides as a reference. If the shortcut is hidden, look under Settings > Visibility > Piano Roll.

### Resize lanes and use the pattern overview
Keywords: piano roll minimap scroll zoom velocity lane splitter
Guide: piano-overview
Drag the splitter above the lower event lane to give it more space. Double-click the splitter to hide or show the lane.
Use the overview scrollbar to move through a long pattern. Zoom in for small edits and out to check the whole phrase.

### Edit velocity and note expression
Keywords: velocity pan release note properties ghost notes scale key event
Guide: piano-expression
Use the Piano Roll lower lane to change velocity and other available note properties. Select notes first when you only want to change part of a phrase.
Use ghost notes for reference to other channels and key or scale guides to see usable pitches. Drag the lane splitter to make the editing area taller.

### Select and edit several channels
Keywords: rack marquee multi select group rows drag mute solo reorder pitch pan volume delete clone
Drag across the channel selectors to select a range. Shift-click selectors to extend a range. On a selected row, mute, solo and supported Pitch, Volume or Pan moves apply to the selection. Pitch, volume and pan edits preserve relative values within the selected group where the control range permits. Delete removes the selected channels. Check the project Undo history before making a second large edit.
Ctrl-drag a row to reorder it. If it belongs to the selection, move the selected rows together and use the insertion line as a guide. The Channel Rack Options > View > Auto resize choice offers Off, Minimum height and Maximum height. A rack with more rows than fit can scroll.
Use Clone for another channel or Clone (Complete) when you need independent pattern material and supported automation, plugin state and matching Playlist placement. Review the clones and their routing before changing shared effects.
Related: Route selected channels to the Mixer | Program steps and patterns

### Stop overlapping drum samples with cut groups
Keywords: closed open hi hat choke cut itself cut self cut by cut groups sample polyphony overlap
For a single sample channel, right-click its Channel Rack name and turn on **Cut itself**. In the Sampler wrench tab, **Cut self** sets matching **Cut** and **By** numbers for that same channel. A new hit releases earlier voices in that group.
For an open hi-hat that stops when a closed hi-hat plays, set **By** on the open hat to a group number. Set **Cut** on the closed hat to the same number. Leave **Cut** at zero on the open hat if it should leave other voices alone. Zero leaves that side of the group unassigned. Test with overlapping playback and check the sound's release setting if it still rings.
Related: Play and shape samples in ML Sampler | Program steps and patterns

### Route selected channels to the Mixer
Keywords: mixer assignment group routing consecutive destination selected rack channels wheel drag miniature
Select the instrument or sample rows first. Open the routing command from a channel or Mixer-number context menu and choose a shared insert or consecutive destinations. Consecutive routing follows rack order. Automation channels are skipped.
Wheel the Mixer number on a selected channel or drag that number to an insert to route the selection together. On an unselected channel the same gesture routes just that row. Escape cancels a routing drag. Locked inserts or an insufficient run of destinations reject the full group change. Undo and project saving include the routing change.
Guide: rack-route
Related: Route audio with the miniature Mixer picker | Set levels, pan and routing

### Manage and render patterns
Keywords: patterns find empty rename color time signature transpose clone split channel render replace
Use Patterns > Find next empty to start a pattern and Rename and color to identify it. Clone when you want a variation. Select in playlist locates occurrences, and Open in project browser finds the pattern in the Browser. Move up and Move down reorder patterns. Insert one, Delete, Set time signature and Transpose act on the chosen pattern.
Use Split by channel to break a combined pattern into channel parts. For audio, choose Quick render as audio clip, Render as audio clip for options, or Render and replace to substitute the result in the arrangement. Keep the editable pattern if you expect to change its notes later.
Related: Program steps and patterns | Build an arrangement | Freeze or bounce a track

### Edit note properties and the event editor
Keywords: double click note properties velocity pan release fine pitch Mod X Mod Y probability event lane
Double-click a note to open its properties and edit timing, length, pitch and supported per-note values. Use the lower event editor to edit values across several notes, such as velocity, pan, release, fine pitch and modulation. Choose the property from the target menu and check the selection before drawing a series of changes.
Use the lane divider to reveal more of the event editor. Change the vertical pitch zoom independently when you need more room between notes. Individual plugins determine which note-expression values affect their sound. Use the MPE editor and a compatible instrument for per-note MPE expression.
Guide: piano-expression
Related: Draw, move and edit notes | Edit per-note expression with MPE

### Use the Piano Roll editing tools
Keywords: piano roll menu draw paint delete mute slice select zoom snap glue strum flam arpeggiate claw limit randomize flip scale levels
Choose Draw for a single note, Paint for repeated placement, Select for a marquee, Slice to cut, and Mute to silence notes without deleting them. Check the current snap before placing or moving notes. Ctrl/Cmd+Shift-drag draws a selection marquee in supported editing contexts. Right-drag with editing tools can erase.
Open the Piano Roll Tools menu to find note-processing commands. Quantize changes timing or length against the grid or a groove. Strum spreads note starts, Arpeggiate makes a note sequence, Flam creates closely spaced attacks, and Flip mirrors pitch or time. Scale levels, Randomize, LFO and other event tools shape the selected data. Preview the result where the dialog offers it and cancel to keep the original.
Use the Piano Roll snap and scale controls to guide placement. A project key, highlighted scale or ghost-note display can help orient you while editing a different channel. To work against other Playlist parts, use the shared MIDI clip timeline.
Guide: piano-tools
Related: Quantize and shape notes | Work with chord and scale tools | Edit several MIDI clips on one timeline

## Playlist & editing
Overview: Arrange clips and edit performances on the timeline.

### Build an arrangement
Keywords: playlist clips tracks markers patterns drag arrangement
![playlist | Playlist tracks with clips arranged under the time ruler](assets/guide/playlist.jpg)
Related: Use the clips picker and loop previews | Import, trim and fade audio | Attach the Mixer to the Playlist
Guide: clip-picker
Choose a pattern, audio clip or automation source, then place it on the Playlist. Drag a clip to move it and use its edges to trim it. Turn on snap for grid-aligned placement.
Use the Playlist tools to draw, paint, split, mute or select clips. Right-click the ruler for marker commands and use the Arrangement selector to manage versions of the song.

**Follow playhead** is off by default. During playback you can scroll or zoom the Playlist yourself. To keep the playhead in view automatically, use Options > Follow playhead or the matching toolbar button. Right-click the button to choose between continuous scrolling and page advances. Your choice is saved in your profile.

### Drop samples and instruments on Playlist track headers
Keywords: sample instrument track arranger header drop audio FL sampler routing
Drop a single audio file from the Browser or file manager onto a Playlist track header. By default this creates a playable ML Sampler instrument, linked Mixer insert and empty one-bar pattern. Play the selected instrument with your MIDI keyboard; use the track's Record button to arm note recording, then press Play. Double-click the pattern to draw or edit notes. The pattern starts at the Playlist play cursor. The original sample is not converted or overwritten. An explicitly saved header-drop choice is respected: As audio clips creates a clip, As audio tracks creates a linked audio lane, and Always ask presents the choices.
Set a persistent choice in Playlist menu > Edit > Drop audio on track headers. Cancel or Escape closes the question without changing the project. Each completed drop is one Undo step. Track links, sample paths and patterns are saved with the project. Existing clips and instruments are retained. Locked, frozen and folder headers reject the operation, and recording must finish first.
Dropping a Browser instrument on a header creates a linked instrument track and pattern directly. Dropping audio on the timeline still creates audio clips. Later audio imports on a linked audio lane use that lane's Mixer output. Use an empty lane when creating a separate instrument or audio part. Multiple sample files can be dropped on the timeline onto separate lanes.
Instrument tracks show a keyboard along the left edge of their header, including instruments added from the attached Mixer's instrument button. The marker grows with track height. A Sampler with a loaded sample shows a waveform symbol instead. Both glow with live MIDI/typing notes and playback note activity. Click this marker to open the instrument's own editor; the separate Piano Roll shortcut still opens the Piano Roll. Right-click the marker, or use its dropdown arrow when several instruments are assigned, to select the instrument for MIDI and editing. Other instruments and their existing clips remain intact. Dropping several samples together or one after another onto the same header creates separate Sampler instances and keeps them in this selector. Undo reverses a batch drop in one step. Explicit header-drop preferences still apply.

The miniature keyboard lights the individual notes you play, including black keys and chords; all octaves share the same miniature octave. A subtle halo around the channel header follows audio activity, including release tails, without lighting extra keys. In **Settings > Theme > Open Theme Editor > Playlist keyboard & activity**, customize idle white/black keys, solid played-key colors, the frame, separators, sample waveform colors, and the halo color, strength (0-200%) and visibility. The live preview shows idle keys and a chord. Apply saves the theme; Cancel restores it, and Undo/Redo, per-field Reset, section Reset and theme export/import also work. Workspace Visibility > Playlist > Instrument keyboard and note activity hides the whole indicator and halo without affecting playback. Its frame and separators keep a consistent pixel thickness as the track grows. At very small heights it becomes a simplified activity glyph.

Choosing an audio input from the track Record button's right-click menu switches that lane to an audio track and hides the instrument marker, while preserving its instruments and notes. Selecting no input restores its instrument role when instruments are assigned. Audio-only and empty lanes have no keyboard marker. Settings > Visibility > Playlist > Instrument keyboard and note activity hides the marker, glow and selector without affecting instruments, routing or recording.
Settings > Visibility > Playlist > Sample drop choices hides the preference menu while preserving the selected drop behavior and all project data.
Related: Import, trim and fade audio | Attach the Mixer to the Playlist

### Use the clips picker and loop previews
Keywords: clip picker pattern picker preview audition loop stop playlist left panel
Guide: clip-picker
In the Playlist menu, enable Show clip picker panel. Its tabs separate patterns, audio and automation. Click a row to choose what to place.
Click the Play icon on the right to loop an audio clip or supported pattern. Click Stop on the same row or start another preview. For a pattern that relies on a hosted third-party instrument, render it to audio before auditioning it here.
Show or hide the preview buttons in Settings > Visibility > Playlist.

### Import, trim and fade audio
Keywords: sample drag wav mp3 audio clip fades crop stretch
Guide: stretch
Drag an audio file from the Browser or your file manager into the Playlist. Use the clip edges to trim, the fade handles to ease in or out, and the clip menu for source commands.
Use Stretch mode when dragging an edge should change the clip's timing. Turn it off to return to trimming. Use the audio channel's settings for pitch or stretching method.

### Make a clip independent
Keywords: make unique clone duplicate shared audio channel route
Use Make unique in the clip menu before changing a source that several clips share. Edits to a shared source can affect other instances.
After making it unique, rename the new source and make the change there. Use a unique audio channel when two instances need separate Mixer destinations.

### Set up an alternate arrangement
Keywords: arrangement duplicate playlist version marker sections
Guide: arrangement
Use the Arrangement selector to manage alternate versions of the Playlist. Keep a separate arrangement for an alternate edit or song structure.
Review which arrangement is active before recording, bouncing or exporting. Patterns and channels can still be shared between arrangements.

### Edit audio timing with warp markers
![warp | Audio Timing with detected transients, an edited anchor and the Sensitivity control](assets/guide/warp.jpg)
Keywords: warp transient flex time guitar rhythm audio timing detect quantize snap
Related: Fit a loop or use its detected tempo | Align multiple performances | Tune vocals in the Playlist
Guide: warp
1. Click the timing-pin button on the Playlist track, or open Edit audio timing from an audio clip's menu. The editor detects transients in the clip.
2. Click a detected line to create a timing marker. Drag anywhere along a movable marker line to adjust timing. Use Snap to follow the selected grid.
3. Choose Select and drag across several points. Shift-drag adds more. Use Quantize to move the selection toward the grid, and choose Grid and Strength before quantizing.
4. Use Sensitivity to control transient detection. Click the value to choose 0–100%, or wheel over it for 1% adjustments. Higher values detect quieter attacks; 50% keeps the original detection threshold. Changes filter the detected markers immediately after analysis and are remembered per profile. They do not move or remove timing anchors already edited. Use Detect transients to rerun analysis, then Quantize when ready. Settings > Visibility > Playlist can hide the sensitivity control without changing its value.
5. Use Ctrl + wheel to zoom time, Alt + wheel to scale the waveform display, and Fit to see the whole clip. Changes render automatically after you release a drag or finish a command. Close keeps completed edits; Escape during a drag cancels that gesture. Updating means the last successful audio is still playing until the new render is ready.

### Fit a loop or use its detected tempo
Keywords: detect sample tempo loop bpm stretch fit make it fit project tempo
Open an audio clip's menu and choose Detect sample tempo. The dialog displays the detected BPM. Choose Yes to set the project tempo to that value, or Cancel to keep the current tempo. The tempo change supports project Undo.
To keep the project tempo and fit the clip to it, use the Stretch / fit options. Compare automatic fitting against half-time, double-time, a known source BPM, or a specified number of bars or beats. Check both the start and end of the loop against the grid.

### Separate stems from a clip
Keywords: stem separation vocals drums bass other model download retry
Open a Playlist audio clip's menu and choose Stem separation. Select the stems you want to create. On first use, confirm the download of the optional engine and model. Setup requires an internet connection and several gigabytes of space. Processing runs locally after setup.
The finished stems are separate audio files and Playlist tracks. Keep the project data folder with the session when sharing it. If setup fails, choose Stem separation again to retry. Details are written to setup.log in the Stem separation folder.

### Align multiple performances
Keywords: vocal align vocalign dub guide timing takes imported vocals instruments
Select at least two audio clips, including the guide, then right-click the intended guide and choose Vocal Align to this clip.
Click Analyze and compare the guide, original and aligned views. Adjust Tightness and listen to the preview before applying. Imported clips work as well as recorded takes. Matching phrasing and comparable source sections give better results.
Use a guide that already has the timing you want. Check sustained, quiet or poorly matched sections by ear.

### Tune vocals in the Playlist
Keywords: pitch correction vocal tuning melody notes formant tune
Guide: vocal
Click the microphone icon on the track with the vocal clip to expand the tuning editor. Let it detect notes, then use the editing tools to select notes and adjust pitch or timing where supported.
Pitch edits and timing share the original recording and remain independently editable. Use Undo/Redo pitch edits or Undo/Redo timing for the active clip, or Ctrl+Z/Ctrl+Y for project history. Copies and slices inherit edits and then become independent. Undo timing on a slice keeps its timeline boundaries fixed. Up to 64 steps per layer are saved with the project. Closing an editor keeps completed edits; Escape cancels the current drag. Workspace Visibility can hide editor and clip-history controls without changing playback.

Fresh playback, export, and consolidation wait for current audio edits. A failed update retains previous audio and offers Retry. Existing rendered projects remain playable; older baked edits can only be recovered when their original sources and metadata are available.

### Comp and organize takes
Keywords: take lanes comping record vocals alternatives
Guide: comp
Right-click the Playlist track and choose Create take lanes / comp when available. Keep alternate performances on lanes under the same parent.
Use the comp tool to choose sections for the main take. Audition transitions and adjust boundaries or fades before bouncing a final take.

### Keep a session editable while reducing CPU
Keywords: freeze defrost unfreeze resume editing CPU instrument tails bounced comp sidechain
Right-click the Playlist track header and choose **Freeze track (save CPU)**. ML Studio renders the track's unmuted audio and pattern clips through their instruments into one audio stem per used Mixer input. The header shows FROZEN. Mixer effects, routing and sidechains continue to run live. Instruments shared with other tracks may still use CPU.
Choose **Unfreeze track (resume editing)** to restore the original clips to the track's current row. Unfreeze before recording into the row or choosing a different comp take. Changes made to the temporary frozen clips are replaced by the original clips. Save the project with its Rendered files so the frozen track can play after reopening.
Related: Freeze or bounce a track | Read signal levels, latency and plugin load

### Commit insert and bus processing to audio
Keywords: wet bounce bounce and replace new track render effects buses external sidechain disk tails stems undo
Right-click a Playlist track header and open **Bounce audio**. **Bounce audio in new track** puts the rendered clip below the source or its full take-lane group and mutes the source. Unmute that row to hear the originals again. **Bounce and replace track** replaces its audio and pattern clips on the same row while retaining automation, take lanes and mute state. Undo brings back the original clips.
Both choices render insert processing and outgoing buses, preserve song position and automation, and include tails. The new audio plays through the live Master. Wet bounce reports an error if the track depends on an external sidechain. Use Freeze to keep that sidechain live. Rendered files go in the project's Rendered folder. Cancelling before the render is committed leaves the Playlist intact.
Related: Freeze or bounce a track | Keep a session editable while reducing CPU | Create sends, buses and sidechains

### Freeze or bounce a track
Keywords: render new track replace cpu bounce audio freeze unfreeze
Guide: track-menu
Right-click a Playlist track header. Freeze track renders the track to save CPU and allows you to return to the editable sources with Unfreeze.
Use Bounce audio > Bounce audio in new track to keep the source on its own track, or Bounce and replace track to work with the rendered clip in place. Save first and compare the result, especially when effects and routing are important.

### Arrange with groups, markers and zoom
Keywords: playlist overview minimap group clips ruler zoom scroll
Guide: playlist-overview
Use the overview above the Playlist to navigate. Drag the thumb to move the view and drag its ends to change the horizontal range.
Group related clips when they should move together. Use time markers to label sections and, where applicable, set start, loop or skip behavior. Use the Playlist snap choice before moving an entire section.

### Select, group and move Playlist tracks
Keywords: folder grouped tracks drag header selection collapse multi-track marquee height reorder
Click or drag down the track names to select rows, including empty lanes. Shift-click extends the selection. Ctrl-click on Windows/Linux or Cmd-click on Mac toggles individual tracks. Ctrl/Cmd+Shift-drag in the timeline draws a marquee around clips.
Right-click a selected track header and choose Create folder. Only selected rows move into the new folder in their existing order. Collapse the folder to save vertical space, expand to edit its children, or dissolve it from its menu. Drag a row or selection by the header handle to reorder it or place it in a folder. An insertion line shows the destination between tracks, and an outline marks a destination folder. Settings > Visibility > Playlist > Track drag destination hides these indicators while keeping drag-and-drop available. Use track groups for related lanes and keep take lanes with their leader.
Resize track heights to show more waveform or automation detail. The Playlist retains track order, names, colors and grouping with the arrangement. Undo can restore structural edits.
Guide: track-menu
Related: Arrange with groups, markers and zoom | Comp and organize takes

### Edit several MIDI clips on one timeline
Keywords: edit selected MIDI clips together ghost notes outlined parts midi import Playlist pattern placement
Select pattern clips in the Playlist and choose Edit selected MIDI clips together. Their notes appear on a shared song timeline. Click the outlined notes of another clip, or use the MIDI parts menu, to make that clip editable. The editor uses the clip's position, trim and pattern speed.
Drop a MIDI file onto the Playlist grid to create a pattern and place it at the selected lane and snapped beat. For a full import, choose File > Import > MIDI file and review the import settings. Undo a mistaken drop before reimporting it.
Related: Draw, move and edit notes | Import and export MIDI files

### Normalize audio and remove silence
Keywords: normalize peak RMS LUFS clip gain batch strip silence threshold attack hold release lookahead
Select audio clips, open Normalize audio clips and enter a Peak, RMS or integrated LUFS target. The operation adjusts each clip's gain individually. Reset sets selected audio clip gains to 0 dB. Clip gain handles apply a relative change when you drag one of the selected clips.
Open Strip Silence to trim quiet starts and ends or split internal gaps. Easy calculates a level for each clip. Manual lets you set open and close thresholds, attack, hold, release and look-ahead. Pick split or trim-only behavior, check the result and use Undo if speech, attacks or tails were cut too closely.
Fades and automatic crossfades smooth transitions where clips overlap. A gain adjustment changes playback level. Use an audio editor or a rendered copy when you want a new sample file.
Related: Import, trim and fade audio | Clean up audio in Tesla

### Process clips, render edits and keep source audio
Keywords: audio processing clip FX reverse make unique stretched automation gain edit sample render bake
Open an audio clip's menu for its processing and editing actions. Use Make unique before changing a shared source if other placements must retain their existing sound. Use Make unique as sample when you need a separately saved audio file.
Clip gain, fades and timeline trimming are Playlist edits. Audio Timing and Vocal Tuning have their own apply and undo paths. Applying one to an already rendered result can bake the previous edit into the new audio. Read the confirmation before combining them.
When an ARA editor is active, render its edit to ordinary audio before applying unsupported clip-processing operations. Keep the project's Rendered folder and referenced audio when moving or archiving the session.
Related: Make a clip independent | Edit audio timing with warp markers | Tune vocals in the Playlist | Use an ARA audio editor

### Add effects to a single audio clip
Keywords: clip effects clip FX single clip only reverb delay reverse automation independent per clip insert bypass
Right-click the intended audio clip and choose **Clip effects...**. Click **+ Add effect** in an empty slot, choose an effect, then click its name to open it. The chain processes the clip before its usual Mixer route. Other clips that share the sample keep their own playback, and the sample file stays intact.
Use **On/Off** to bypass a slot or **Effects on/Bypassed** to compare the entire chain. **Up** and **Down** reorder the effects, **Params** exposes the effect's parameters, and **Remove** clears the slot. Page arrows reveal more slots. Use **Copy chain** on the first clip and **Paste chain** on the destination to reuse it.
Save the project to retain the chain. Bounce an ARA clip to ordinary audio before adding Clip Effects. If the app reports no room, check the available Mixer inserts because Clip Effects use a dedicated internal insert.
Related: Process clips, render edits and keep source audio | Insert effects, wet/dry mix and plugin bypass

### Control playback, looping and navigation in the Playlist
Keywords: follow playhead auto scroll loop song selection time marker play start overview scroll zoom
Drag in the ruler to make a time selection. Use the play-start marker to choose a return point and the horizontal overview or scrollbar to navigate far away from it. Change Playlist snap for accurate clip moves at the current zoom.
Follow playhead scrolls during playback to keep the playhead in view. Turn it off to edit a distant passage while the song continues playing. The right edge can scroll as you drag, paint or place material. Scroll or zoom first when precise placement is easier at a closer view.
Use time markers to label sections and to return to song positions. Use arrangements for alternate versions of the timeline within one project.
Related: Build an arrangement | Set up an alternate arrangement | Arrange with groups, markers and zoom

## Mixer & routing
Overview: Route signals, build effect chains, and master a mix.

### Set levels, pan and routing
Keywords: mixer volume pan master signal flow insert route
![mixer | Mixer with insert inspector, channel strips and meters](assets/guide/mixer.jpg)
Guide: mixer-level
Route a Channel Rack instrument or audio clip to a named Mixer insert. Use its fader and pan controls to balance it, and check its meter.
Right-click a Mixer channel to rename it or choose Change color > Custom color. The custom swatches accept six-digit hex colors. Settings > General > Input offers optional double-click reset for faders and knobs and opening main menus on hover. Both start off.
Effects in the insert's slots run in order. Check that the track reaches Master through a direct route or a bus. Avoid clipping the Master if you are exporting a mix.

### Send a Mixer track to an interface output
Keywords: hardware output cue headphone mix out extra stereo pair asio
Open a Mixer track and click Hardware out below its EQ. Choose an additional pair exposed by the active audio device, such as Outputs 3 / 4, or choose Off. The output gets the track's post-fader signal alongside its normal sends. Master plays through the device's main outputs.
The choice is saved with the project for that device and affects live playback and monitoring. For a cue mix, send the desired tracks to a bus and set that bus's hardware output. A stereo-only device offers Off. Settings > Visibility > Mixer can hide the selector while retaining the assignment.

### Route audio with the miniature Mixer picker
Keywords: route picker send to mixer track track number audio clip channel rack
Guide: rack-route
The Channel Rack automatically fits visible channels when samples or instruments are added. It can grow upward to use the available workspace and keeps its width. When the channels exceed the screen height, use the vertical scrollbar. A manual size is retained on ordinary repaints and when reopening the app, until the channel list changes. Channel Rack Options > View > Auto resize offers Minimum height (fit channels), Maximum height and Off; the choice is remembered per profile.

In the Channel Rack, Shift-click the channel selectors to add or remove channels, or drag a selection across rows. Right-click a selected channel or its Mixer number and choose Route selected channels to mixer. Choose one destination for all, or consecutive mixer tracks starting at the chosen insert. Scrolling the displayed Mixer number on a selected row assigns that number to every selected channel; a control on an unselected row affects only that row. Dragging a selected row's Mixer number onto the Mixer also routes the selection. Routing preserves destination names, skips automation channels, and refuses the whole group if a track is locked or there are not enough consecutive inserts. Each change supports Undo/Redo. Escape cancels the picker or drag. Settings > Visibility > Channel Rack can hide the routing menu without changing routing.
Click the Channel Rack's Mixer track number or the channel window's TRACK box to open the miniature Mixer picker. For a Playlist audio clip, use Send to mixer track in its menu.
Search by insert name or number and click a strip to assign the route. Press Escape to cancel. Clips sharing the same source also share its route, so use Make unique first when they need separate assignments.

### Create sends, buses and sidechains
Keywords: send triangle reverb aux bus route sidechain compressor ducking
Guide: mixer-send
Select the source Mixer track. Click the routing triangle beneath the destination track to create a send, or right-click it for routing choices. Use the send level to control the amount.
For parallel effects, keep the dry route and send some signal to a reverb or delay return. For a bus, use Route to this track only if the bus should carry the full signal.
For sidechain ducking, create a sidechain route and choose that source in the effect slot's Sidechain input menu. Check the plugin's own sidechain controls.

### Duck a track with Quick Sidechain
Keywords: kick bass duck pumping quick sidechain ML Limiter threshold ratio release silent send no effect
1. Select the Mixer track that should trigger the ducking, such as the kick.
2. Right-click the routing arrow under the track you want to duck, such as the bass, and choose **Quick Sidechain**.
3. Open **ML Limiter** on the destination to adjust the result. It starts in COMP with the external input selected, a threshold of −30 dB, a 4:1 ratio and a 200 ms release.

The trigger keeps its current outputs and gains a sidechain-only send. ML Studio enables destination effects and adds the limiter without reordering existing effects. If the destination does not duck, check the trigger meter, the destination's FX switch, global All FX bypass and the limiter's external input. A full chain or routing loop prevents the setup. Undo reverses the operation.
Related: Create sends, buses and sidechains | ML Limiter | Insert effects, wet/dry mix and plugin bypass

### Choose the Mixer layout and built-in strip
Keywords: full mixer custom strip traditional gate tame comp drive low cut high mid low
Guide: mixer-layout
Use the Mixer view buttons to choose Full or Traditional. In Full Mixer, right-click an insert and choose Channel layout > Custom strip to see its built-in EQ, Low Cut, Gate, Drive, Comp and Tame. Vox adds vocal controls.
Standard (FL-style) emphasizes the meter. The overall Mixer view and each insert's channel layout are separate choices.

### Attach the Mixer to the Playlist
![mixer | Mixer inspector and channel controls](assets/guide/mixer.jpg)
Keywords: attach detach integrated mixer inspector instrument track recording bus arrangement
Related: Set levels, pan and routing | Create sends, buses and sidechains | Freeze or bounce a track
Guide: mixer-layout
Click Attach to Playlist at the top left of the Mixer. The Playlist opens at its current size, with source lanes in mixer order and a compact inspector for the selected channel. Settings > Visibility > Playlist has separate controls for the inspector and the Mixer’s Attach to Playlist button. Buses and Master stay in the inspector instead of occupying recording lanes. The separate Mixer stays open. Switch between Mixer and Playlist with the workspace buttons when their windows overlap. Both views control the same channels and effects.
The larger red button beside each source lane's mute/solo light arms that track. Instrument lanes arm MIDI notes; audio lanes use their chosen input. Right-click the red button to choose an audio input, then press transport Play to record. Selecting or arming a lane does not start recording by itself. Vocal tuning and Time Warper remain on each lane.
Attached lanes use their mixer channel colors. Every source lane has a Piano Roll icon: left-click opens its instrument's Piano Roll (an empty lane gets an empty sampler); right-click opens the instrument menu for that mixer channel. Enable Keyboard right-click opens Plugin Picker in that menu or in Playlist Options for direct access to the picker. The choice applies to attached and ordinary Playlist track controls. The inspector lists outgoing sends in their destination colors below the effects. Adjust a send with its knob and use the arrow to inspect the destination. A mixer track receiving audio from another insert is treated as a bus. Separators mark visual groups.
On enables or mutes the entire mixer channel; FX bypasses only its effects. The Playlist lane mute controls the clips on that lane, so it can differ from muting the mixer channel and everything routed through it.
The attached inspector defaults to the Traditional bottom-fader layout for every Mixer view. Right-click the inspector button or open Mixer Options > Attached inspector layout > Inspector style to choose Full channel strip. The choice is saved in your profile. Both include the same editable parametric EQ used by the Mixer, with Open EQ for its full editor. Changing views never creates another EQ instance or changes channel processing. Use the small panel button beside + above the track headers to hide or show the inspector. Drag the grip below EQ, Insert Effects, or Sends to resize it. The fader and strip fill the remaining height. Short windows can temporarily reduce the space given to the upper sections. Click the selected channel name to choose another channel, including Master. Click the Insert Effects or Sends heading to collapse it, and click again to restore its size. Drag the scrollbar thumb or wheel over either list when it overflows. Right-click the inspector button or open Mixer Options > Attached inspector layout to change section order, show controls, or reset the layout. Sizes, order, and collapsed sections save automatically for each mixer view and apply to every channel. Settings > Visibility > Attached mixer inspector has the same show/hide controls. Theme Editor > Attached mixer inspector controls the background, grips, scroll thumbs, and Traditional fader color. Send names use the destination colors. Clicking a send name keeps the current channel selected; use its arrow to inspect that destination. The native Full Mixer VU switches between VU and peak when clicked; the Traditional inspector uses a stereo peak meter. Both the layout choice and visibility are saved per profile. The Mixer EQ visibility setting also controls the inspector EQ.
Use the + above the track headers to insert an instrument. This creates its mixer channel and routes the instrument to it, as one Undo action. Existing routed instruments appear automatically. Mixed-instrument patterns and mixed-output rows remain ordinary arrangement lanes; attaching does not split or reroute them.
Drag a track header up or down, or use **Move up** and **Move down** in its right-click menu. This order is saved per arrangement. **Insert one** adds an empty track above the selected track and creates its mixer insert. **Delete track** removes the lane and its clips from the arrangement. Its mixer insert, plugins and routing remain available in the Mixer. Use Undo to recover the lane and clips.

While the Mixer is attached, drawing, painting, importing, pasting or moving audio onto a linked lane uses that lane’s mixer output. Moving one placement never reroutes other copies of its source. The app creates an independent source when needed while reusing the recording file, including its audio settings, automation and pitch/timing edit history. Dragging onto a locked or frozen lane is rejected. Press Escape during an attached clip move or paint gesture to cancel; a completed edit is one Undo/Redo action.

Right-click an attached track header and choose **Track type and routing**. **Audio track output** assigns the lane’s audio to an existing mixer insert. **Instrument track** chooses the specific instrument used by the Piano Roll and recording controls. **Free arrangement track / Unlink** stops automatic routing on that lane and keeps its existing sound. **Link tracks using existing routing** adopts unambiguous existing audio or instrument assignments without changing the sound; mixed tracks remain unchanged. **Arrange tracks by Mixer order** is a separate undoable action. Track headers keep their familiar appearance without extra routing labels.

Patterns keep their original instruments when placed or moved. To intentionally use another linked instrument, right-click a single-instrument pattern and choose **Use this track’s instrument**. This makes an independent pattern before changing its notes. Mixed patterns must first be split by channel; patterns with event automation must have that automation separated before retargeting. **Inspect current clip outputs** in the track menu lists the actual destinations, including clips whose routing differs from the lane. Audio with private Clip Effects or an ARA editor retains its existing routing, with a status explanation, so its processing is not bypassed.

To paste copied clips while attached, select the destination track and place the playhead at the start position.

Detach from Playlist restores the previous Playlist row order and the usual unattached editing behavior, while retaining new recordings, edits and routing changes already made. New audio imports again use the ordinary Master default, existing clips retain their source routing when moved or placed, and ordinary paste keeps its original rows. Reattaching restores the saved attached associations and order without rerouting clips that were edited while detached. A project created in the attached view keeps its arrangement when detached. Saving remembers the attached layout and new lane assignments. Settings > Visibility > Playlist can hide the inspector or track record buttons without changing routing, recordings or arming.
To use these track controls without attaching the Mixer, enable Settings > Visibility > Playlist > Playlist record and instrument buttons (off by default). The red button arms an instrument for notes or opens the audio input selector; right-click it to choose an audio input explicitly. Press transport Play to record. The piano button opens the track's Piano Roll, creating an empty sampler if needed; right-click it to choose an instrument. A new instrument or audio recording track gets its own mixer channel when needed. Cancelling a picker creates nothing. Track bindings survive save/reopen and hiding the buttons; instrument insertion and recording-track creation support project Undo/Redo.

### Lock a Mixer channel
Keywords: lock padlock fader protect routing effects
Guide: mixer-lock
Click the padlock on a Mixer channel to protect its configuration from accidental manual changes. Click it again when you want to edit the channel.
Playback and existing automation continue while it is locked. If the icon is missing, look under Settings > Visibility > Mixer.

### Rename, color and group mixer channels
Keywords: rename mixer channel custom color hex separator channels
Right-click an insert and choose Rename or Change color. In the color menu, choose a swatch or open Custom color to enter a six-digit hex color. Attached Playlist lanes use their corresponding mixer channel colors.
Use separators to mark mixer sections visually. The mixer shortcuts scroll to the corresponding sections. Use buses to sum audio and sends to feed parallel effects.

### Add, reorder and bypass effects
Keywords: plugin picker effects slot vst eq insert replace bypass
Guide: effect-slot
Click an empty effect slot and choose an effect. Click a loaded slot to open its controls. Use the slot menu to replace, move, copy or remove an effect where offered.
Use the slot enable switch to compare the effect with the dry signal, and check the channel level after changes that add gain.

### Save and reuse effect chains
Keywords: effect chains plugin picker save load favorites import export
Guide: effect-chain
Open the effect picker and choose Effect Chains. Right-click a Mixer insert and choose Save effect chain to capture its ordered effects and settings. To build one in the picker, Ctrl-click the effects you want and choose Save as chain.
Click a chain to select it, Ctrl/Cmd-click to toggle individual chains, or Shift-click to select a range. Click Choose to load them onto the selected Mixer insert, or drag a selected chain to the Mixer insert you want. Dragging a selected card carries the whole selection. Chains load in the order shown, with their effects in order, into consecutive free slots. One Undo removes the whole insertion.
Use the available rename, duplicate, favorite, import and export controls to organize and share chains. Recipients need the plugins used by the chain.

### Mix with ML EQ
Keywords: eq bell shelf low pass high pass dynamic spectral tame deharsh listen solo
![ML EQ | ML EQ editor](../crates/os-app/assets/plugin-picker/eq.png)
Guide: effect-slot
Add ML EQ to an effect slot. Select a band and adjust frequency, gain and Q. Use the band type controls for a bell, shelf or cut.
Use the band card to bypass or listen to an individual band. Open the band's Dynamic or Spectral controls for level-dependent changes and audition before committing to deep cuts.
The Mixer mini EQ and an EQ plugin in a slot are separate processors. Check which one is enabled when troubleshooting a tonal change.

### Use the Mixer mini EQ before or after effects
Keywords: mini EQ Open EQ pre post insert effect order EQ On Off bypass EQ spectrum knob
Select a Mixer track and use its mini EQ graph to adjust the channel's parametric EQ. **Open EQ** opens the full editor for the same settings. Adding ML EQ to a regular effect slot gives you a separate instance.
Click **Pre** or **Post** to move the built-in EQ before or after the entire user effect chain. For example, Pre can remove rumble before it reaches a compressor. Post can shape the finished sound after compression. **EQ On/Off** bypasses or re-enables this channel's EQ and retains the curve. If the button is hidden, show **Settings > Visibility > Mixer > Mini EQ bypass button** and make room for it by widening the panel if needed.
Related: Mix with ML EQ | ML EQ controls and processing modes | Insert effects, wet/dry mix and plugin bypass

### Use dynamics, saturation and time effects
Keywords: limiter compressor noise gate clipping reverb delay drive transient chorus
Guide: effect-slot
Use a compressor to reduce level changes, a gate to control quiet sections, and a limiter or clipper to control peaks. Match the output level before comparing an effect with bypass.
Use drive and saturation for harmonics, Transient Processor to shape attack and sustain, and chorus or stereo effects to adjust width. Put reverb or delay on a send when several tracks should share it.

### Correct headphone monitoring
Keywords: headphones phones autoeq monitor correction profile
Guide: phones
Click the Master strip's headphone icon, search for the exact model, and choose the matching profile. Match the impedance, pads or ANC setting if the catalog lists variants. Selecting a model downloads and activates its parametric correction. If the download fails, the previous correction stays active. The downloaded profile works offline.
Use the **PHONES** switch to compare or bypass correction. Its initial state is off. This is a listening preference saved per profile and applied at ML Studio's device output. It includes live instruments and Browser previews, works independently of Master bypass, and is excluded from audio exports, freeze, bounce and Mixer recording. Use PHONES to bypass while listening through speakers.
Related: Read signal levels, latency and plugin load | Export mixer stems and choose what gets processed

### Master a mix with Master Assistant
Keywords: master assistant loudness reference headphone correction
![ML Master Assistant | ML Master Assistant editor](../crates/os-app/assets/plugin-picker/master-assistant.png)
Guide: master-assistant
Start with a balanced mix and leave room at the Master output. Open Master Assistant from the Master strip and let it analyze representative playback before comparing its settings.
Set a target that fits the delivery format and check that the result is not clipping. Use Detailed controls to inspect or adjust EQ, multiband compression, stereo width and limiting. Compare at similar loudness before exporting.

### Choose a channel strip and use its processors
Keywords: traditional wide full mixer console SSL style channel strip dynamics filter compressor gate EQ tape saturation
Choose a Mixer view from its layout menu. Compact views prioritize faders and insert count. Expanded and Full views expose more strip controls, while Traditional and the Vintage workspace have their own arrangements. On an attached Mixer, choose the optional Full channel strip or use the compact inspector.
On a strip with built-in processing, enable the sections you need and adjust filters, EQ, gate or compression while watching levels. Use tape drive for color, stereo separation to control width, and the output fader to balance the result. Drag or wheel supported controls, or use their menus to enter/reset a value and create automation.
For processing order, use Mixer effect slots and drag effects to arrange them. Use slot enable, the track's effects switch, and the strip's own enable switches to isolate a stage during troubleshooting.
Related: Choose the Mixer layout and built-in strip | Add, reorder and bypass effects

### Fill Mixer inserts with a channel strip
Keywords: fill all channels fill all insert channels with channel strip view copy rack nothing happens locked full effect slot Waves bulk load
Switch the Mixer to **Channel Strip** view and use the **+** or strip menu on an insert. Open **Fill all insert channels with** and select an eligible installed plugin to create fresh instances on the insert channels. To copy settings from an existing strip, open its menu and choose **Copy this rack and settings to all inserts**. Each destination gets an independent instance. The Master is outside this workflow.
Stop recording first. Every destination needs an empty user effect slot and must be unlocked. One locked insert or one full chain can prevent the whole operation. If the picker reports that it is still checking the plugin, wait for the check to finish. If a load or editor check fails, the error names the problem and the existing assignments stay intact. Check the insert effect slots to confirm a fill, and use project Undo to reverse it.
For a single insert, choose **Choose channel strip**, or use **Use an existing effect** to dock a compatible instance already in its effect slots. **Remove strip (keep bypassed effect)** restores the built-in strip and keeps the effect in its slot in bypass.
Related: Choose a channel strip and use its processors | Use SSL 360 Link in Channel Strip view | Lock a Mixer channel

### Use SSL 360 Link in Channel Strip view
Keywords: SSL 360 Link VST3 wrapper third party channel strip mapping setup scan Solid State Logic
Install the free SSL 360 Link VST3 from Solid State Logic separately. Open Settings > Plugins > Find new plugins. In Mixer > Channel Strip view, click a strip's + or open its strip menu, then Choose channel strip > Solid State Logic > SSL 360 Link. It appears after a successful scan. The setup article remains available from the strip menu even before installation.
In Link, open Configure Link > PLUG-INS, scan for your installed VST3 effects and load one. Use a factory map when available, or make a user map in Link. Link hosts VST3 effects, not instruments. Factory maps cover selected plugins; arbitrary effects are not automatically mapped. See SSL's SSL 360 Link User Guide for its mapping workflow.
The Mixer displays Link's actual editor and controls. Its full interface remains accessible, including its plugin browser and mapping pages. Use Link's zoom control if needed. ML Studio's built-in strip knobs are not automatically mapped to the wrapped plugin. Available controls, metering and processing depend on Link and the loaded effect.
To configure in a separate window, choose Open strip in plugin window; its D button detaches it for another monitor. Choose Dock original interface here to bring it back. Use an existing effect can dock a Link instance already in that insert's effect slots.
Save the project to retain the wrapper's saved state. Copy this rack and settings to all inserts creates independent plugin instances using the wrapper's state; Fill all insert channels with starts fresh instances. These operations need a free effect slot on each destination. Remove strip (keep bypassed effect) restores the built-in strip and keeps the effect bypassed. Undo restores the assignment. Keep Link, the hosted plugin and any user maps installed when moving projects between computers.
Settings > Visibility > Mixer > SSL 360 Link in Channel Strip picker hides the picker entry and setup shortcut without changing loaded effects or project data.
Related: Find installed plugins | Choose a channel strip and use its processors | Insert effects, wet/dry mix and plugin bypass

### Insert effects, wet/dry mix and plugin bypass
Keywords: FX slot plugin order 64 expandable effects bypass wet dry multi add move effect smart disable
Select a Mixer insert and click an empty effect slot to load a plugin. Open the plugin from a filled slot, then adjust its own mix or the slot's wet/dry control. Keep an eye on output level when comparing bypassed and processed sound. Use the insert effects switch to compare the whole chain.
Open an effect window and expand its host controls using the gear button if necessary. Click ON to bypass or re-enable that exact Mixer effect slot. The Mixer slot indicator and plugin window stay synchronized, and Undo restores the change. The insert-wide FX switch can still bypass the entire chain. Locked inserts reject changes. Settings > Visibility > Plugin windows > Effect bypass button hides this button without changing processing.
Use the effect slot menu to replace, remove, move or save an effect preset. Extra slots appear in the expanded effects list. Use the effect chain commands when you want to move or reuse a whole chain. The Plugin Picker can insert several compatible effects in one operation.
An insert has 63 user effect slots and a dedicated mini EQ. Fill the last visible slot or use **+ Add slot** to reveal another empty one. Wheel over the list to scroll after it grows beyond ten rows. Alt+wheel over a slot moves the effect. Slot moves skip the mini EQ, which stays before or after the whole chain according to its pre/post setting.
Smart disable allows eligible idle effects to sleep. If a plugin needs to generate sound or track time while idle, compare playback with Smart disable off under Settings > Audio > Advanced > CPU.
Guide: effect-slot
Related: Save and reuse effect chains | Audio processing and recording settings

### Read signal levels, latency and plugin load
Keywords: meters rms peak clip gain staging pdc compensation performance monitor smart disable
Watch the insert and Master meters before adjusting individual processors. Reduce the upstream gain if a stage is clipping. A downstream fader can lower the output without undoing clipping inside an effect.
ML Studio compensates playback paths for latency reported by hosted plugins. Large look-ahead, oversampling and linear-phase processing can increase monitoring delay. Use the plugin performance monitor to locate heavy instruments and effects, and bypass a suspect processor while reproducing a dropout.
If an effect has a heavy UI, compare playback with its window closed. Freeze tracks when instrument processing is the bottleneck. Bounce when you want to commit mixer processing into a new audio file.
Related: Fix delay, crackles or no sound | Freeze or bounce a track

## Browser & plugins
Overview: Find sounds, manage plugins, and organize presets.

### Find installed plugins
Keywords: plugin scan vst vst3 vst2 clap plugin manager add folders missing plugin
Related: Search, favorite and manage the plugin picker | Load and save presets | Recover a missing plugin or preset
Guide: plugin-scan
Open Settings > Plugins. Add the folders where your plugins are installed, then click Find new plugins. Use Rescan all when the inventory needs rebuilding.
Use a plugin format and architecture that match your system. After a scan, find instruments in Add or the rack picker, and effects in a Mixer slot's picker.

### Search, favorite and manage the plugin picker
Keywords: f8 plugin picker favorites blacklist installed filter
![picker | Plugin Picker tiles, filters and search](assets/guide/picker.jpg)
Open the picker from the rack or an effect slot to search for the right type of plugin. F8 opens the general picker. Check the target before choosing, since an effect goes to a Mixer insert and an instrument goes to the rack.
Use the installed and favorites filters to narrow the list. If a plugin is missing, check its blacklist status and scan results in Settings > Plugins.

### Recover a missing plugin or preset
Keywords: missing plugin wrong format vst3 clap preset blacklisted
Guide: plugin-scan
Check the plugin name and format the project expects. Install that version when available, add its folder to the plugin manager, and run a scan.
If an effect slot opens with a missing plugin, preserve a copy of the project before replacing the slot. Compare the replacement's sound and reconnect automation where needed.

### Load and save presets
Keywords: preset vstpreset fxp fxb ospreset patch bank browse previous next
Use the arrows or preset menu in the plugin wrapper's title bar to step through presets. The menu offers browsing and saving your own presets where supported.
Drop a compatible preset onto the intended plugin window or slot to load it there. Use Save preset as to preserve an edited version before switching to another preset.
Host-loaded state presets prepare while the current sound continues playing. When ready, the selected instrument or effect switches with a short smoothing ramp; a rejected preset leaves the current one in place. Held notes restart with the new sound, and routing and automation stay connected. Existing effect tails and a plugin's internal voice state are not carried across. Changes made inside a plugin's own preset browser remain controlled by that plugin.

### Use an ARA audio editor
Keywords: ara melodyne plugin audio edit compatibility
Install a compatible ARA editor for your platform. Open it through the audio clip's ARA action where available and let it read the clip.
Complete pitch or timing edits in the editor, then save the project. Some clip operations, including inline warp, require rendering the ARA edit first.

### Browse samples and release history
Keywords: browser folders sample preview presets plugin downloads changelog
![browser | Samples Browser with library actions](assets/guide/browser.jpg)
Guide: browser
Use the Browser tabs to find samples, presets, projects and plugin downloads. Add your own folders in Settings > File to keep libraries easy to find.
Use the sample preview controls to audition files before placing them. Open Help > Change Log to browse or search the bundled release history offline.

### Discover free plugins
Keywords: plugin downloads favorites freeware open source portable compatibility installation

Open Browser > Plugin Downloads. Show opens filters for Favorites, Native download, Portable install, Open source and Freeware. Sort switches between categories and alphabetical names. Search matches every word across names, descriptions and licences. Reset filters and search returns to the full catalog.

Click the star beside a plugin to save it to your profile's download favorites. These are separate from favorites in the installed Plugin Picker. Plugin Downloads shows verified packages for your platform. Click the download arrow to get the official installer or archive directly. Use the Plugin suites category to browse bundles. Complete the installation or extract the plugin archive, then click Scan Plugins.

Direct downloads are checked against the catalog checksum before opening. Cancel stops the current download. Downloads opens the package folder. Hide the discovery controls under Settings > Visibility > Browser > Plugin Downloads filters and favorites. Hiding them shows the unfiltered category browser and preserves your saved favorites and choices. Hiding the whole tab does not remove any installed plugins.

### Browse samples, favorites and the Starter Pack
Keywords: browser samples favorites search tags folders preview volume loop normalize start pack download cc0
Open the Samples tab and use Add my sample folder or drag a folder into the Browser. Search and filter your library, star items for Favorites, and audition a highlighted file with Right Arrow. Preview volume and its assigned Mixer route affect listening while browsing. Drag the sound to the Channel Rack to play it as an instrument or to the Playlist to arrange audio.
Use Download ML Studio Starter Pack for the optional collection of 263 CC0 WAVs, including drums, loops, keys, vocal hits and textures. The download starts from that button, can be cancelled, and appears as a Browser library after verification. The samples and download cache together take about 570 MiB.
Dropping audio files into the Samples Browser adds references to their current locations. Keep the original files available. Add or remove library roots under Settings > File. Use the Browser's menu to detach it for a second monitor or dock it back into the main window.
Guide: browser
Related: Browse samples and release history | Discover free plugins

### Restore a moved Browser sample folder
Keywords: browser sample tree library disappeared missing folder drive moved relink samples factory content
Open Settings > File > **Browser extra search folders** and add the sample library's current location. You can also drag the folder into the Browser or right-click empty Browser space and choose **Add sample folder**. If you use a removable drive, confirm that it is mounted at the same location before refreshing the Browser.
Adding a library folder makes its contents available to browse. For an open project with missing audio, use the project's missing-file prompts to locate the original files. In a project that uses samples from an FL Studio installation, **Settings > File > FL Studio factory content folder** points to that installation. **Reset** returns to the detected installation.
Related: Browse samples, favorites and the Starter Pack | Diagnose missing sound, files and plugins | Send a project with its audio files

### Browse the current project and plugin parameters
Keywords: browser current project history patterns generators effects used samples initialized controls presets parameter list
Use Current project to locate patterns, generators, effects, samples, initialized controls, project data and Undo history. The tree provides project context when you are trying to identify which item is in use. For reusable sounds, use the plugin database and preset folders instead.
From a hosted plugin's wrapper menu, choose Browse parameters to see host-exposed parameters in the Browser. Use their context actions for automation or controller linking. If a control is missing, check whether the plugin publishes it to the host.
Related: Load and save presets | Create an automation clip | Edit curves and manage targets

### Use Community Samples and the voting board
Keywords: community samples audition upload account votes feature request voting closed genre sound type license
Open Community Samples in the Browser, choose a sound type or genre, and audition a result before dragging it into the Playlist. Community browsing and downloads require a connection. Use your community account to vote or upload.
For an upload, supply the sound type, genre and creator credit, and review the royalty-free license before accepting it. Keep the permission to share the source sound. On the suggestions board, Voting closed means the item is visible and its voting buttons are disabled.
Use the feedback page for bug reports. If you are reporting a reproducible issue, include the exact build and the steps that show it.
Related: Report a problem and check for updates | Browse samples, favorites and the Starter Pack

### Host VST3, VST2, CLAP and built-in plugins
Keywords: plugin format VST3 VST2 CLAP scan paths architecture instrument effect wrapper editor D detach keyboard focus
Install a plugin that matches the operating system and processor, then scan its installation folder from Settings > Plugins. Add instruments to the Channel Rack and effects to Mixer slots. The plugin's native format and identity matter when reopening projects.
Open the wrapper menu for its preset, parameter, compatibility and processing options. The D control switches between an attached editor and a separate window. Keyboard focus and typing-piano priority determine whether unmodified keys play notes, go to the plugin, or run DAW shortcuts.
A missing plugin can leave a saved slot or channel that cannot process. Check the format and scan error first. Preserve the project before substituting another plugin. Use a render to share the sound with someone who lacks the original plugin.
Related: Find installed plugins | Recover a missing plugin or preset | Use an ARA audio editor

## Stock instruments
Overview: Learn the built-in samplers and synthesizers.

### Play and shape samples in ML Sampler
Keywords: sample sampler root note loop start end envelopes reverse pitch
![ML Sampler | ML Sampler editor](../crates/os-app/assets/plugin-picker/sampler.png)

Load a sample into a sampler channel. Set the start and end, pitch or root note, and loop behavior to suit the sound. Open Piano Roll and play notes at different pitches to check it.
Use the channel's envelopes and filters to shape playback. Keep long files on disk when supported if you need to reduce memory use.
For repeated notes, chords from one key and changes tied to velocity or key position, open the wrench tab.
Related: Route ML Sampler to the Mixer | Set up Sampler arpeggios, echoes and tracking | Use sample metadata and loading preferences

### Set up Sampler arpeggios, echoes and tracking
Keywords: sampler arp echo wrench note effects repeat held notes chord time gate feed ping pong fat mode velocity key
Open ML Sampler's **wrench** tab. If the note effects are hidden, show **Settings > Visibility > Plugin windows > Sampler note effects**. Each sampler channel saves its own values.
#### Arpeggiator
Pick a **direction** to enable note cycling. **Time** controls the interval and at maximum holds the first note. **Gate** controls the length of each generated note. **Range** spans octaves, **Repeat** repeats each step, and **Slide** enables pitch transitions. **Held notes** cycles the notes you play together. **Note alone** uses the played pitch and its octave range. A named chord or scale generates pitches from the lowest held key.
#### Echo
Raise **Feed** to hear repeats. **Time** sets their spacing and **Echoes** sets the count, up to ten. **Pan**, **Pitch**, **Mod X** and **Mod Y** change the repeated notes. **Ping pong** alternates stereo position and **Fat mode** produces closely spaced stereo repeats, even at zero Feed. Set **Echoes** to zero to stop scheduling new repeats. Echoes already scheduled finish with the settings they started with.
#### Tracking
In **Vol** or **Key**, use **Mid** as the reference velocity or note position, then adjust **Pan**, **Mod X** or **Mod Y** to make the sound respond above and below that point. Try two velocities or two pitches that straddle Mid so you can hear the difference. Wrench note-effect changes update live during playback.
Related: Play and shape samples in ML Sampler | Choose between written arpeggios and live Sampler arpeggios

### Route ML Sampler to the Mixer
Keywords: sampler track mixer insert destination master right click drag wheel

Click **TRACK** in the ML Sampler window to open the miniature Mixer picker. Choose a destination by name or number, or scroll the mouse wheel over **TRACK** to change it one insert at a time. **0** is Master. With the default controls, you can also drag **TRACK** onto a visible Mixer track.

To change track numbers by dragging, right-click **TRACK** and turn on **Drag up/down to change Mixer track**. Hold the left mouse button on the number and drag up for higher track numbers or down for lower ones. The number updates as you drag. Release to apply the route as one undoable change, or press **Esc** while holding to cancel. A quick click still opens the picker, and the wheel still steps through tracks. This preference is off by default and saved in your profile.

The same right-click menu can open the picker, show the assigned track in the Mixer, or route the sampler to Master. You can hide the menu under **Settings > Visibility > Plugin windows > Sampler Mixer track menu**.
Related: Play and shape samples in ML Sampler | Route audio with the miniature Mixer picker

### Design sounds with ML Wave
Keywords: synth oscillator wavetable modulation lfo envelope unassign
![ML Wave | ML Wave editor](../crates/os-app/assets/plugin-picker/lemon-wave.png)
Add ML Wave as an instrument and choose a preset as a starting point. Change oscillator and filter settings while playing a few notes, then shape the sound with the envelope.
Assign an LFO or envelope to a control to add movement. Right-click a modulated control to manage or remove its assignment. Save a new preset before changing patches.

### Design sounds with ML Plasma
Keywords: ml plasma gridline wavetable synthesizer oscillator modulation preset
![ML Plasma | ML Plasma editor](../crates/os-app/assets/plugin-picker/ml-plasma.png)
Add ML Plasma from the instrument picker. Use its oscillator and wavetable controls to shape the source, then use the filter, envelopes, and modulation to shape the movement and tone.
Save the project to keep the synth state. Use the instrument's preset controls when you want to reuse a sound in another project.

### ML Trio
Keywords: trio synthesizer oscillator citrus sytrus fm presets
![ML Trio | ML Trio editor](../crates/os-app/assets/plugin-picker/trio.png)
Use ML Trio to layer three oscillators. Start with one waveform. Bring in a second oscillator for octave weight or light detune. Use the third for an additional layer and rebalance each oscillator as the sound changes.
Choose a custom shape to use a sample as one period of the oscillator. Load the sample from the oscillator or sample page. Adjust tuning, stereo balance, envelope and filter, then save a preset for another project.
Related: ML Citrus | Load and save presets

### Slice audio with ML Chop
Keywords: chop slices sample beat slicer pads
![ML Chop | ML Chop editor](../crates/os-app/assets/plugin-picker/chop.png)
Load an audio sample into ML Chop and check its detected slices. Adjust slice boundaries before playing them from pads or MIDI notes.
Use the slices to build a pattern in Piano Roll. Compare playback with the original loop to check the trigger positions and tempo.

### ML Citrus
Keywords: citrus sytrus FM RM operators filters matrix carrier modulator envelope harmonics
Citrus is a six-operator synth. Choose a factory preset, then use the routing matrix to send an operator to the output, a filter, or another operator. A carrier reaches the audio output. Turning up an operator routed into the carrier changes its harmonics.
Work on one operator at a time. Shape its waveform and envelopes, set frequency ratio or offset, then route it. The filter and effects sections shape the combined output. Compare a simple carrier patch with a routed modulation patch to hear the difference.
Citrus can interpret supported Sytrus state in imported FL projects. Play the original and imported versions side by side when migrating a song.
![ML Citrus | Citrus operators and modulation matrix](../crates/os-app/assets/plugin-picker/citrus_default.png)
Related: ML Trio | Load and save presets

### Program sampler playback, loops and file options
Keywords: sample reverse pitch time stretch keep on disk preload trim loop metadata tempo root note envelope declicking
Load a sample in ML Sampler and audition it at its root note. Use the sample page to set start, length, reverse, loop behavior, stretch and declicking. If the file supplies tempo or a root note, the appropriate General > Advanced read setting can use it on load. Confirm both when importing a mislabeled file.
Use the envelope and filter controls to shape amplitude and tone. The channel's misc settings include polyphony and note effects. If a shared sample also appears in the Playlist, make an independent source before changing only one version.
Keep on disk streams compatible uncompressed WAV or AIFF samples. Reverse, certain sample effects and stretch modes can require RAM. The sampler indicates the active mode. Drag the edited waveform into the Browser when you want a reusable WAV in Favorites.
Related: Play and shape samples in ML Sampler | Fit a loop or use its detected tempo

### Build a patch in ML Wave
Keywords: wavetable oscillator unison warp phase harmonics filter modulation matrix envelope LFO presets Night Shift keyboard arpeggiator
Start with a factory or user preset. Use oscillator wavetable selection and position to choose a tone, then adjust pitch, unison and available oscillator shaping. Set the filter and amplitude envelope while playing a phrase. Use tempo-synced modulation when the movement should stay on the song grid.
Assign an envelope or LFO to a supported target and adjust its modulation amount. Inspect an assigned control to edit or clear the assignment. Use the on-screen keyboard, scale-aware chords and arpeggiator to try the sound. Mouse and incoming MIDI can both play the instrument.
The Night Shift factory bank and categorized preset browser provide starting points. The wavetable tools can build and audition tables from marked recordings. Save the patch under a new name to reuse it. Content shown in a preset or library browser depends on the installed content.
![ML Wave | Oscillators, filter and modulation in ML Wave](../crates/os-app/assets/plugin-picker/lemon-wave.png)
Related: Design sounds with ML Wave | Load and save presets

### Play slices and edit their boundaries in ML Chop
Keywords: sample slicer slice markers threshold pads waveform BPM audition export
Load a loop into ML Chop, adjust the slice boundaries, and audition individual slices before playing the part from MIDI or the Piano Roll. Moving a boundary changes the material triggered by the corresponding slice. Use the waveform to check that a cut starts at the intended attack.
When building a kit from a rhythm loop, try playing the slices in their original order first. Use snap and the source tempo to establish a close match, then rearrange the notes. Save the project with the source sample available.
![ML Chop | Slice controls and waveform in ML Chop](../crates/os-app/assets/plugin-picker/chop.png)
Related: Slice audio with ML Chop | Fit a loop or use its detected tempo

## Stock effects & editors
Overview: Look up each ML effect and audio editor.

### ML Tesla time stretch and pitch shift
Keywords: Tesla audio editor duration pitch semitones Signalsmith stretch
![ML Tesla | ML Tesla editor](../crates/os-app/assets/plugin-picker/tesla_empty.png)

In Tesla, select the audio to change, or clear the selection to process the whole sample. Open Tools > Time stretch / pitch shift, or click the stretch tool. Duration changes the length independently of pitch: 200% is twice as long; 50% is half as long. Pitch uses semitones: +12 is one octave up. Click either value to type an exact number, or use its minus and plus buttons. Duration supports 25–400%; pitch supports -24 to +24 semitones.

Apply renders in the background and shows Processing. Cancel or Escape keeps the original audio. Wait for a cancelled render to finish before opening another. On success the processed range is selected; Undo restores the previous audio and markers in one step. Source files on disk are never overwritten. Save As or send the edited audio to the Playlist when ready. Markers, regions and loops follow the time change; whole-sample changes also adjust stored sample tempo. The tool supports samples and results up to 20 minutes. Stop recording before processing.

Hide its toolbar button and Tools menu entry in Settings > Visibility > Toolbar extras > Tesla time stretch and pitch shift. Existing audio and applied edits are preserved.

### ML Tape
Keywords: Lemon Tape Airwindows ToTape6 tape saturation soften head bump flutter

Add ML Tape in a Mixer effect slot. Input drives the tape model (-12 to +12 dB); Soften rounds the high frequencies; Head bump adds low-frequency resonance; Flutter introduces pitch movement. Output trims the processed level and Mix blends dry and processed audio. Start with Gentle Warmth or Soft Mix Bus. Match output loudness when judging the effect. Drum Color is more driven; Worn Cassette adds obvious softness and movement. The reel artwork illustrates the effect, not an audio meter.

Knobs support typing values, reset, automation and the usual plugin preset saving. Existing effects and old projects keep their original sound. Lemon Tape uses Chris Johnson's MIT-licensed Airwindows ToTape6 audio processor with Lemon controls; credit and licence are included. This is a creative tape model, not a claim to reproduce one specific physical machine. At very high sample rates, flutter depth is bounded to fit the processor's delay line. Below 22.05 kHz, internal coefficients use a 22.05 kHz floor for stability.

Hide it from new-plugin menus with Settings > Visibility > Toolbar extras > Lemon Tape in plugin menus. Existing instances still play, save and reopen. Plugin Manager's existing Show control also remains available.

### ML Hall
Keywords: hall, reverb, Dragonfly, Freeverb3, early reflections, damping, pre-delay

Add **ML Hall** in a Mixer effect slot for a stereo hall with early reflections and a modulated reverb tail. It is an additional effect; existing ML Reverb instances keep their original sound. Try Vocal Hall or Short Drum Space first. On a dedicated send, use Hall Send - Fully Wet; adjust the channel's send amount to blend the effect.

Decay sets the tail length (0.20–12 seconds). Pre-delay separates the dry attack from the whole wet signal (0–250 ms). Early Balance blends early reflections at 100% with the diffuse late tail at 0%; start around 25%. Diffusion softens the tail's texture. Low Cut and High Cut filter the wet signal, leaving the dry signal intact. HF Decay sets how quickly high frequencies fade relative to the main decay: lower values give a darker tail. Modulation adds movement, Width sets stereo spread (0% is mono), and Mix blends dry and wet. Changes are smoothed; moving Pre-delay can briefly bend the wet pitch, so stop or use gentle changes if unwanted.

Drag a knob, use its wheel, or right-click for the standard value, reset and automation options. Save your settings in a plugin preset or project. The display is a settings illustration, not a live audio meter. Hall geometry and modulation speed are fixed; this is a focused Lemon interface to the Dragonfly/Freeverb3 hall engine, not every control from the original plugin.

To hide it from new plugin menus, turn off **Lemon Hall in plugin menus** in Settings > General > Workspace Visibility > Choose what to show > Toolbar extras. Saved instances still load and play. This option is saved with your profile.

### Clean up audio in Tesla
![ML Tesla | Tesla audio editor](../crates/os-app/assets/plugin-picker/tesla_empty.png)
Keywords: cleanup, clean up, denoise, noise reduction, noise profile, hiss, hum, Tesla, apply to track

Right-click an audio clip in the Playlist and choose **Audio processing > Clean up audio in Tesla**. Tesla opens a copy of the clip's current source audio. It includes the full source file, so a trimmed clip can show more audio than its visible Playlist section. The clip's position, length, fades, gain, stretch settings and routing remain in the Playlist.

For the best result, close the cleanup panel, select at least 100 ms of background noise with no wanted voice or instrument, then open **Tools > Clean up audio (denoise)** and click **Learn selected noise**. Tesla remembers the noise profile for this editor session and clears the selection so the next preview cleans the whole sample. To clean only a portion, close the panel, select that portion and reopen Cleanup. If there is no noise-only section, use **Automatic estimate** and listen carefully; it is an estimate, especially on continuous music.

Start with 12 dB Reduction and 1.50x Sensitivity. Higher Reduction permits stronger attenuation; higher Sensitivity identifies more audio as noise. Click **Preview cleaned** to render and listen. A progress bar appears while processing; Close/Escape cancels processing without editing the sample. Use **Original**, **Preview cleaned** and **Stop** to compare. Preview never changes the document or saved project. Tesla temporarily suppresses its incoming mixer audio during these previews so it does not double up with the audition.

**Apply cleanup** commits the result inside Tesla with one undo step. If you continue editing afterward, the yellow **Apply to track** button at the bottom of Tesla (also in its File menu) returns the edited sample without running denoise again. **Apply to track**, available only when opened from a ML Studio clip, writes a separate cleaned WAV and updates that exact Playlist clip automatically. Other occurrences of the same sample are unchanged. Playlist Undo restores the original clip in one step; the original audio file is never overwritten. The cleaned WAV stays in the project's Rendered/Cleaned folder (or the normal user render folder for projects without a data folder) and is referenced when saving the project. Keep these files with the project when transferring it.

Cleanup uses the current source file. Previously rendered vocal tuning or audio timing is already in that file and stays baked into the cleaned copy; the copied clip does not keep a live link to those earlier editing sessions. Mixer effects are not baked. ARA clips must first be rendered to ordinary audio. If you change the Playlist, source file or source channel while cleanup is open, Apply refuses to replace anything: reopen Cleanup from the intended clip. Changing Tesla's sample length or sample rate also prevents returning it to that clip, because doing so would invalidate its timing. Return links end when the project closes or the operation succeeds.

Tesla also works separately as a **VST3** plugin in FL Studio and other VST3 hosts. Load or drag an audio file into Tesla, use the same cleanup controls, then **Apply cleanup** and **File > Save sample as**. Saved plugin state includes the edited audio. Automatic **Apply to track** is a ML Studio connection; external hosts use Tesla's normal save or drag-out workflow. This is an audio editor that processes the loaded sample, not a live noise gate or an automatic multitrack editor.

This spectral tool targets steady noise such as hiss, fans and hum. It does not separate competing voices or reliably remove individual clicks. Strong settings can damage wanted audio, so compare before applying. Supports mono/stereo, 8–384 kHz, up to 20 minutes and 120 million samples. Noise profiles are not saved across editor sessions. Hide its toolbar tool and Playlist menu action through **Settings > General > Workspace Visibility > Choose what to show > Toolbar extras > Tesla audio cleanup / Playlist cleanup action**; hiding does not change existing audio.

### ML EQ controls and processing modes
Keywords: ML EQ parametric EQ 2 analyzer seven bands dynamic spectral de-harsh phase HQ gain match audition
Use the seven-band overview or the graph to select a band. Drag its node left or right for frequency and up or down for gain. Ctrl-click overview headings to select several bands. The band menu and Band controls drawer expose filter shape, bandwidth or Q, mode and exact values. Use the band power switch to bypass that band.
Regular mode applies a fixed curve. Dynamic mode changes gain according to the signal crossing its threshold. Use attack and release to decide how quickly the gain reacts. Spectral dynamics and de-harsh target narrower activity inside the selected range. Audition a band or the affected signal to locate the problem, then compare in context.
Use A/B, temporary RMS Level match and the analyzer to compare settings. Reset and Invert gain apply to selected bands. Auto fit changes graph scale. The analyzer's Speed menu controls its display, including range, resolution, tilt and averaging where offered.
Quality choices include HQ and linear-phase modes. Modes with reported latency participate in Mixer playback compensation. The overview and floating band card have independent ML EQ visibility switches.
![ML EQ | EQ graph, seven-band overview and analyzer](../crates/os-app/assets/plugin-picker/eq.png)
Related: Mix with ML EQ | Read signal levels, latency and plugin load

### ML Limiter
Keywords: limiter compressor gate ceiling lookahead attack release sustain gain sidechain
Use the limiter for peaks or the compressor for level changes over time. Set gain and ceiling while watching the input/output and gain reduction. Shorter timing responds to transients faster. More look-ahead can smooth limiting and adds processing delay.
In compressor mode, set threshold, ratio, knee and timing. To duck a track, send a sidechain to the insert, choose the detector source in the plugin or use Quick Sidechain where available, then check that the reduction follows the trigger. Use the gate controls to reduce low-level signal between events.
![ML Limiter | Limiter and compressor display](../crates/os-app/assets/plugin-picker/limiter_0.png)

#### Parameter names
Gain, Soft saturation threshold, Limiter ceiling, Limiter attack time, Limiter attack curve, Limiter release time, Limiter release curve, Limiter peak window, Comp threshold, Comp ratio, Comp knee, Comp attack time, Comp release time, Comp curve, Comp RMS window, Noise gain, Noise threshold, Noise release time.

Related: Create sends, buses and sidechains | Insert effects, wet/dry mix and plugin bypass

### ML Reverb
Keywords: reverb room size decay pre-delay early reflections diffusion damping dry wet
Add the reverb to an insert for a local space or to a send for a shared space. Room Size, Decay and Diffusion shape the tail. Pre-delay keeps the source attack separate from the first reflection. Use the frequency and damping controls to keep the return from masking the dry sound.
On a send, lower the dry level and blend the amount from each track with its send. On an insert, blend within the plugin or use the slot's wet/dry control.
**Low cut** and **High cut** shape the frequency range entering the reverb. **High damping** governs how quickly the high frequencies die away in the tail. **Crossover** sets the split below which **Bass multiplier** changes the decay. Use **Early reflection level** for the first reflections, **Wet level** for the tail and **Dry level** for the unprocessed source. **Mod Speed** and **Mod Depth** set the rate and strength of movement. Use **Tempo-based predelay** when you want pre-delay expressed in steps and ticks.
![ML Reverb | Reverb timing, filtering and level controls](../crates/os-app/assets/plugin-picker/reverb_default.png)

#### Parameter names
Low cut, High cut, Predelay, Room size, Diffusion, Decay time, High damping, Bass multiplier, Crossover, Stereo separation, Dry level, Early reflection level, Wet level, Mod Speed, Mod Depth, Tempo-based predelay, Mid / side input.

Related: ML Hall | Create sends, buses and sidechains

### ML Delay
Keywords: delay echo feedback stereo ping pong tempo sync modulation filter diffusion wet dry
Set delay time against the song tempo for rhythmic echoes, then use feedback to decide how long the echoes last. Select the delay mode for the stereo behavior you want. Use filtering to make repeats darker or thinner and modulation to give them movement.
On an insert, blend the repeats with dry sound. On a send, keep the delay wet and blend with the send level. When exporting, choose a tail setting that keeps the last repeats or wraps them if you are making a loop.
![ML Delay | Delay timing, feedback and filtering controls](../crates/os-app/assets/plugin-picker/delay_default.png)

#### Parameter names
Input wet, Delay type, Tempo sync, Keep pitch, Time, Delay time R/L offset, Time smooth, Stereo spread, Modulation rate, Modulation time, Modulation cutoff, Diffusion level, Diffusion spread, Distortion type, Feedback level, Filter mode, Feedback cutoff, Feedback resonance, Distortion, Distortion knee, Distortion symmetry, Rate reduction, Bits reduction, Output wet, Output tone, Output dry.

Related: Export settings, metadata and stems | Create sends, buses and sidechains

### Set up a tempo-synced stereo delay
Keywords: ML Delay tempo sync ping pong keep pitch time smooth note division feedback filter repeats wet dry
Open **ML Delay** on a Mixer insert. Enable **Tempo sync**, set **Time** against the song grid, and choose **Ping Pong** to alternate repeats between the two sides. The **Delay time R/L offset** control becomes **Delay time R/L pan** in Ping Pong and pans the input. **Stereo spread** adjusts separation. Play a short isolated note to hear the rhythm.
**Feedback level** determines how much signal returns for another repeat. Use **Filter mode**, **Feedback cutoff** and **Feedback resonance** to shape successive repeats. **Keep pitch** and **Time smooth** affect how the effect responds to time changes. On a send, turn down **Output dry** and set **Output wet** so the return contributes the delayed sound.
Related: ML Delay | Create sends, buses and sidechains | Export a seamless loop or keep effect tails

### ML Chorus
Keywords: chorus modulated delay voice rate depth stereo spread wet
Use Chorus to widen or thicken a sustained part. Start with a shallow depth, then adjust rate, delay and stereo spread while listening in the mix. Greater depth and rate make pitch movement more obvious. Use the mix control or the Mixer slot amount to blend it.
Check a stereo chorus in mono before committing a wide setting. If a sound loses focus, reduce the depth or wet amount.
![ML Chorus | Chorus modulation and stereo controls](../crates/os-app/assets/plugin-picker/chorus_default.png)

#### Parameter names
Delay, Depth, Stereo, LFO 1 speed, LFO 2 speed, LFO 3 speed, LFO 1 wave, LFO 2 wave, LFO 3 wave, Cross type, Cross cutoff, Wet only.

Related: ML Stereo Enhancer | ML Stereo Shaper

### ML Soft Clipper
Keywords: soft clip threshold post gain saturation peaks drums clipper
Lower the threshold to round more of the incoming peaks. Set Post level so the output compares at a similar loudness to the bypassed signal. Use it on drums or a bus when you want rounded transients and the resulting distortion.
Adjust the input and threshold together. Compare the output in context, particularly if another processor follows the clipper.
![ML Soft Clipper | Threshold and Post controls](../crates/os-app/assets/plugin-picker/soft_clipper_default.png)

#### Parameter names
Threshold, Post gain.

Related: ML Fast Distortion | Read signal levels, latency and plugin load

### ML Fast Distortion
Keywords: fast dist drive threshold pre post distortion saturation mix
Use the input or pre control to drive the waveshaping stage. Set the threshold and distortion type, then trim the output and blend dry/wet to suit the source. Try a small amount on a bass or drum bus before increasing the drive.
Distortion creates new harmonics and can change level substantially. Check the output meter and compare with the bypassed signal before the next effect in the chain.
![ML Fast Distortion | Drive, shape and level controls](../crates/os-app/assets/plugin-picker/fast_distortion_default.png)

#### Parameter names
Pre amp, Threshold, Type, Mix, Post gain.

Related: ML Soft Clipper | ML Tape

### ML Balance
Keywords: utility gain volume balance pan automation
Use Balance for a simple gain or stereo balance change inside an effect chain. It is useful for automation when the Mixer fader also serves as your mix level, or for trimming the signal going into the next plugin.
Place it before a dynamics or saturation effect to change that effect's input, or after it to adjust the result. Use the parameter's context menu to make an automation clip or controller link.
![ML Balance | Volume and balance controls](../crates/os-app/assets/plugin-picker/balance_default.png)

#### Parameter names
Balance, Volume.

Related: Create an automation clip | Set levels, pan and routing

### ML Stereo Enhancer
Keywords: stereo enhancer separation width phase offset pan gain
Use Stereo Enhancer to change separation and channel balance. Adjust phase offset carefully, then listen in stereo and mono. Larger offsets can make the signal wider but can also cause partial cancellation on a mono device.
Set output level to make a useful bypass comparison. A centered bass, kick or lead may need a smaller change than pads or background textures.
![ML Stereo Enhancer | Separation, offset, pan and level](../crates/os-app/assets/plugin-picker/stereo_enhancer_default.png)

#### Parameter names
Pan, Volume, Stereo separation, Phase offset, Phase offset placement, Inversion.

Related: ML Stereo Shaper | ML Chorus

### ML Stereo Shaper
Keywords: stereo shaper matrix left right crossfeed phase invert difference output
The four matrix controls set how much of each input channel goes to each output channel. Use the diagonal for the direct left and right paths and the cross paths for blending or swapping sides. Negative values invert polarity on the corresponding path.
Check stereo and mono after a change. Use delay and phase controls only when you want that relationship to change. A configured difference output can route an additional signal in the ML Studio Mixer.
![ML Stereo Shaper | Left/right routing matrix and phase controls](../crates/os-app/assets/plugin-picker/stereo_shaper_default.png)

#### Parameter names
Right into left, Left, Right, Left into right, Delay, Phase, Effects position.

Related: ML Stereo Enhancer | Create sends, buses and sidechains

### ML Transient Processor
Keywords: transient processor attack release sustain shape drums split wet dry
Use the transient control to emphasize or soften an attack and the decay or release control to change the tail. Start with one sound such as a snare. Increase attack for definition or shorten the tail to leave room for the next hit.
Set the effect amount, listen at similar output loudness, and check that softer notes still behave as intended. Use the slot wet/dry blend for parallel transient shaping.
![ML Transient Processor | Transient and tail shaping controls](../crates/os-app/assets/plugin-picker/transient_processor_default.png)

#### Parameter names
Attack, Drive, Release, Gain, Attack shape, Release shape, Split frequency, Split balance, Processing.

Related: ML Limiter | ML Soft Clipper

### ML Pulp Filter
Keywords: pulp filter Love Philter multi filter envelope pattern cutoff resonance series parallel
Pulp Filter combines filter units with modulation and pattern control. Start with one unit and pick its filter type. Adjust cutoff and resonance while listening, then shape the envelope or sequence and check its trigger and tempo behavior.
Turn on additional units for more complex movement. Check the routing and output levels when combining units. Compatible imported Love Philter state uses the project's existing filter settings.
![ML Pulp Filter | Filter and modulation controls](../crates/os-app/assets/plugin-picker/pulp_default.png)
Related: Create an automation clip | ML EQ controls and processing modes

### ML Press
Keywords: press Maximus multiband dynamics compression limiting crossover low mid high master saturation curve
Choose Low, Mid, High or Master and shape that band's dynamics with its transfer curve and timing controls. Set the crossover and audition the individual band while tuning it. Use the Master section and meters to check the combined result.
Use gain before or after the dynamics stage to establish how much signal reaches the curve. Make small moves and compare with bypass at a similar loudness. Projects with supported Maximus state can open in Press.
![ML Press | Band controls and transfer curve](../crates/os-app/assets/plugin-picker/press_default.png)
Related: ML Limiter | Master a mix with Master Assistant

### ML Master Assistant detailed controls
Keywords: master assistant EQ dynamic EQ multiband compression imager maximizer reference loudness ceiling
Open Master Assistant from the Master strip. Opening the editor does not enable processing. Use its on/off control to decide whether it affects playback and song export. Use a representative section for analysis or a reference comparison where offered.
Open Detailed controls to edit Equalizer, Dynamic EQ, Multiband Comp, Stereo Imager and Maximizer. Drag an EQ band for frequency and gain, scroll on it for bandwidth, or use the numerical fields. Dynamic EQ has threshold and reduction controls per band. Multiband compression adds crossover, ratio, attack, release and makeup. Imager width is 100% at unity. Maximizer provides threshold, ceiling, release and mode.
Compare the enabled and bypassed master, check the loudest section and audition the export. Applying a new analysis can replace the current curve, so save a version first if you want to return to the hand-edited result.
![ML Master Assistant | Master Assistant editor](../crates/os-app/assets/plugin-picker/master-assistant.png)
Related: Master a mix with Master Assistant | Export settings, metadata and stems

### Record, edit and save audio in ML Tesla
Keywords: Tesla Edison waveform editor record insert audio selection marker region normalize fade sample drag export
Add ML Tesla to a Mixer slot to work on audio from the insert or open it from a Playlist clip when a linked clip action is available. Use its input recording controls to capture audio into the editor. Load an existing WAV when you want to edit a file.
Drag across the waveform to select a range. Use the Edit or Tools menus for the available cut, copy, crop, fade and processing actions. Mark a region or loop to keep its boundaries while auditioning. Compare a tool's preview before applying it and use Tesla Undo to restore the sample.
Save a new sample or drag an edited range into the project. For a linked cleanup session, Apply to track returns a new file to the originating clip. Keep it with the project when transferring the session.
![ML Tesla | Waveform editor and processing toolbar](../crates/os-app/assets/plugin-picker/tesla_empty.png)
Related: Clean up audio in Tesla | ML Tesla time stretch and pitch shift

## Automation & controls
Overview: Automate parameters, map hardware, and edit shortcuts.

### Create an automation clip
Keywords: automation parameter ramp volume filter right click last tweaked
Related: Edit curves and manage targets | Map a MIDI knob or controller
Right-click a supported knob or plugin parameter and choose Create automation clip. For a plugin parameter that does not expose the command directly, use Tools > Last tweaked or Browse parameters to find it.
Place the automation clip in the Playlist and edit its points. Play the affected section and watch the target control to confirm the link.

### Edit curves and manage targets
Keywords: automation points tension curve link move channel track
Add points on the automation line and drag them to change value and time. Use point menus or curve controls for the shape between points.
Moving the clip to another Playlist row changes where it is displayed. Check its parameter link when you want to automate a different control. Use a separate clip or link for an independent target.

### Map a MIDI knob or controller
Keywords: link to controller midi learn multilink hardware
Guide: midi
Enable your controller in MIDI settings. Right-click a supported parameter and choose Link to controller, then move the hardware control when prompted.
Test the full range and check whether the link should apply to the current project or to a reusable control setup.

### Customize mouse and keyboard behavior
Keywords: controls input bindings modifier shortcut marquee right drag wheel
![controls | Controls settings for context, gesture and action](assets/guide/controls.jpg)
Guide: controls
Open Settings > Controls. Choose an editor context, event and modifiers, then search for an action. Use Capture to record a key and try it in that editor.
A more specific context can override a broader one. Use Restore event to return to inherited behavior, or Do nothing to suppress the gesture.
Clicks and drags can have separate assignments. Set a left click to insert a note and left drag to select a region where those actions are available.

### Restore a shortcut that stopped working
Keywords: shortcut override keyboard binding context hotkey typing keyboard
Guide: controls
Check which editor is active and whether a text field or dialog is taking keys. Turn off Typing keyboard to piano if a note key should be used for editing.
Open Settings > Controls and look for a matching event in the editor and its parent contexts. Use Restore event to remove an override and return to inherited behavior.

### Build action sequences and toolbar buttons
Keywords: macro script lsa saved actions custom toolbar import export
Guide: controls
In Settings > Controls, use Append to build a sequence of compatible editing actions. Save a name to reuse it and choose Pin as toolbar button to make it accessible from the toolbar.
Use Export to back up controls. Import accepts a controls JSON file or a portable .lsa action script. Portable scripts run supported ML Studio actions and do not execute system commands.

### Connect an external assistant with MCP
Keywords: AI assistant external MCP tools working orb approval
Open AI assistant and enable AI control for the session. Save the setup to the Desktop and configure it in a compatible MCP client. Approve the named client in ML Studio when it connects. Disable AI control in ML Studio to revoke access.
The blue Working orb shows when an approved MCP command is running. Hide it under Settings > Visibility > Editor feedback > MCP activity orb.

### Record automation and manage initial values
Keywords: record automation Touch Latch mouse gestures VST3 CLAP plugin parameters record filter playlist clips initial values
Right-click **Record**, enable **Automation** in the Notes / Audio / Automation filter, and choose **Automation mode**. **Touch** is the default: move a knob or fader while recording, then release it to let existing automation resume. **Latch** keeps writing the last value after you release, until you stop, pause or disarm recording. Other enabled recording types can run at the same time.
Start playback and move a supported host control or an exposed automatable parameter in a plugin editor. Supported host targets include channel volume, pan and pitch; Mixer faders, pan, separation, EQ, strip controls, sends and slot mix; effect bypass, master pitch and tempo. This does not make every application preference or every private plugin control automatable. A plugin must expose the parameter and report its edits to the host. Plugins without gesture boundaries, wheel edits and typed values use a short 250 ms inactivity interval to end Touch. Latch avoids that timeout.
The take is buffered during movement. Stop, pause or disarm to commit editable **Playlist automation clips** automatically. By default, repeated movements of the same parameter update **one continuous clip**, including later recording passes and recording after save/reopen. Recording another parameter creates its own clip. Existing audible clips for that parameter in the current arrangement are combined, with new gestures replacing only the touched intervals. In Touch gaps, existing clip automation resumes; where no existing clip is active, the last value holds rather than ramping toward the next gesture. Unaffected curves in other arrangements and other targets of shared clips are preserved. A clipped nonlinear segment may be represented by closely spaced points to preserve its shape. One **Undo** restores the entire pre-take state; Redo reapplies the take.
To keep each gesture separate, right-click **Record** and enable **Record as separate clips**. This is off by default and independent of Touch/Latch. With it on, a Touch gesture creates its own clip and Latch continues until the take ends; overlapping existing clip starts may split the take to preserve playback priority. Changing this setting commits any buffered take using the previous setting; it does not reorganize clips until another take is recorded.
Linked channel and Mixer controls place recorded automation on the associated music clip's Playlist row, using the same overlay as **Create automation clip**. The row's automation switch shows or hides the overlay without muting music or stopping automation playback; the automation chooser selects which curve to edit. Unlinked controls use a free row for their first clip, then reuse that row. Muted clips are left alone. Use **Tools > Cancel recording** to discard the uncommitted take. Switch to Song mode to hear Playlist clips if you recorded while in Pattern mode.
During playback, the original host knobs, Mixer faders and exposed generic plugin controls follow their automation values. This display follows the heard playback position and does not record another take or change the project's saved manual settings. Grabbing a control starts from its displayed position; the held control takes priority while recording. Native third-party editors remain responsible for drawing their own parameter controls.
Double-click a recorded automation clip to edit its points, shapes and targets. Save/reopen retains the clips. Saving during recording commits the recorded portion and continues the held or latched gesture. In the default mode, later loop passes replace only the portions touched on those passes within the same clip. The recording mode and separate-clips choice are saved per profile. **Settings > Visibility > Toolbar panels > Automation recording mode** and **Automation clip recording option** hide their respective menu controls without changing saved behavior or project data.
If every usable Playlist lane is occupied or locked, the take stays buffered and the status message asks you to free an unlocked lane and press Stop again. Saving is postponed until those buffered gestures can be placed. Existing initialized controls are kept. With automatic initialization enabled, a missing initial value is added from the available starting value of the control. Inspect **Browser > Current project > Initialized controls** if a control jumps when playback starts.
Related: Create an automation clip | Edit curves and manage targets | Choose a recording mode and lead-in

### Find out why a knob moves when playback starts
Keywords: automation initial value knob jumps reset changes on play initialize controls event editor link target unexpectedly
Check the active arrangement for automation clips linked to the knob. Open the clip's target list and confirm that its curve controls the intended parameter. Pattern events can also drive a control while that pattern plays. Review the event editor for the parameter in any pattern where the jump occurs.
If the knob jumps when the project opens or playback starts, inspect **Current project > Initialized controls** in the Browser. An initial value sets the starting point before later automation takes over. Removing an initial value leaves any existing automation clips and recorded events in place. To avoid adding initial values automatically with future clips, turn off Settings > General > Advanced > **Initialize controls automatically**.
Related: Record automation and manage initial values | Edit curves and manage targets | Use sample metadata and loading preferences

### Reuse linked automation on a Playlist track
Keywords: linked automation follow boundaries extend cut trim restore track automation Move
Create automation from the effect on an insert routed to a Playlist track, or choose an existing track automation from the track menu. A new linked automation can span that track's routed clips. The same curve can extend when you add or lengthen material.
Cutting or shortening an automation clip preserves the edited boundaries. Choosing its saved automation from the track menu can restore it later. Check the target link and active arrangement if a control moves unexpectedly. Use a separate automation clip when several targets need independent curves.
Related: Edit curves and manage targets | Attach the Mixer to the Playlist

### Use MIDI output, sync and the MIDI Monitor
Keywords: clock MIDI out sync input external offset port monitor messages midi mapping troubleshoot
Enable the hardware output in Settings > MIDI and match its port to the output destination. Choose a sync type and set the Master sync offset when ML Studio leads. Enable external sync and set its offset when another device leads. Test start, stop and a short recorded passage with the intended clock source.
Use MIDI Monitor to check which port reports notes, CC, transport or pedal input. Dedicated DAW ports often carry control messages separately from keys. Refresh the device list after connecting hardware and check for another application using the same port.
Related: MIDI devices, controller type and sync | Set up a keyboard or mixer controller

## Workspace & settings
Overview: Set up the app, displays, menus and profile defaults.

### Show or hide interface features
Keywords: workspace visibility remove button toolbar settings tabs clip picker
![visibility | Workspace Visibility groups and search](assets/guide/visibility.jpg)
Guide: visibility
Open Settings > General > Workspace Visibility, or choose the Visibility tab. Expand a section or search for the control, then toggle it.
Search accepts several words across a section and its controls. For example, **plugin sampler** finds the sampler controls under Plugin windows.
These choices save to your profile and follow you between projects. Use the Playlist menu's Show clip picker panel switch to open or close the picker itself.
Restore default visibility brings back the default show/hide choices. Existing toolbar layout settings still apply.

### Open menus by hovering
Keywords: hover menu mouseover flyout mouse pointer submenu opens automatically missing switch visibility two steps
1. Open Settings > Visibility and search for **Open menus on hover**. Show it if it is hidden.
2. Open Settings > General > Input and enable **Open menus on hover**. Visibility makes the setting available. The Input checkbox controls menu behavior and starts off by default.
3. Click a main menu title to start. With the menu open, hover over another main title or a row with a submenu arrow. Click the final command to run it. Escape or a click outside dismisses the menu.

In a context menu, Settings dropdown or detached ML Studio view, open the menu in the usual way first. Hover then opens its supported submenus. Third-party plugin interfaces have their own controls. If hovering stops working, check the Input checkbox as well as the Visibility switch. Hiding the checkbox preserves its current value.
Related: How menus work | Show or hide interface features

### Move the Settings window
Keywords: settings options menu move drag position title header floating
Drag the Settings window by its title bar to move it within the workspace. File, Options and other dropdown menus stay anchored to their menu buttons.
Each Settings page remembers its expanded and collapsed sections during the session. Clicking the selected tab keeps your scroll position.

The User Guide has its own Detach command. Use it to place the guide on another monitor.
Related: Put ML Studio windows on several monitors

### Customize menus
Keywords: menu layout arrange submenu hide items piano roll channel rack mixer
Guide: menus
Open Settings > Menus and choose the menu to customize. Show or hide entries and use the structure controls to reorder them or place them in submenus.
Menu layouts are saved to your profile. Restore the menu's defaults if a command becomes hard to find.

### Show a window or restore your workspace
Keywords: window missing channel rack mixer browser playlist layout
Use View or the toolbar to reopen the missing window. If its shortcut is hidden, use Settings > Visibility to show it.
Toolbar window icons stay highlighted while their windows are open, including windows behind another editor or detached onto another monitor. Several icons can be highlighted at once. Closing a window clears its highlight. Project and Plugin Picker shortcuts also reflect their open state. Settings > Visibility > Toolbar window shortcuts > Open-window highlights can hide this feedback without closing windows or changing the project.
A normal app launch restores the windows left open or closed at the last exit, their sizes and positions, and their stacking order. This includes a workspace with every editor closed. Last-session state takes precedence over the startup template or automatically reopened project. Opening a project explicitly still restores that project's saved layout. Plugin editors are restored only for the matching startup project/template and plugin identity. Closing all editors leaves the Browser as it was; its own visibility choice is also remembered.
A project can restore saved window positions. For reusable workspaces, arrange your windows and choose View > Window layout > Arrange windows > Save custom layout as. Save a named .lslayout file. Saved layouts appear in the same menu; Load custom layout opens one saved elsewhere. Saving again under the same name updates that preset after the file dialog overwrite check.
Detached workspace windows and embedded Channel Strip editors wait until the project's samples and plugins have finished loading before restoring. Their saved positions and sizes are retained, including layouts on another monitor.
Window layouts include Playlist, Mixer, Piano Roll and Channel Rack positions and sizes, open/closed state and stacking order, Browser width and visibility, detached editor frames, Mixer view, Mixer attachment to Playlist, and the positions of the performance monitor, tempo tapper and touch controller. They do not store project-specific plugin editors or replace notes, clips, routing or effects. Offscreen frames are constrained by the existing window recovery rules. Undo last layout change toggles back to the previous workspace. Delete saved layout asks before removing its file. Escape cancels a save/load dialog.
To assign a hotkey, choose Assign layout hotkey in that menu, then the saved layout. Settings > Controls opens with the layout action selected by its search filter. Capture the key combination, review the current assignment, and click Assign. Use Global scope to switch from any editor. Saved bindings lets you change or remove the shortcut later; Restore default removes that override. Each profile keeps its own layouts and shortcuts. Settings > Visibility > Workspace can hide the layout menu while saved hotkeys keep working.

### Change theme, icons and cursors
Keywords: theme vintage fl 2005 theme editor cursor pack toolbar colors
Guide: theme
Open Settings > Theme and choose a skin or theme. Use the theme controls to adjust colors, icons, cursor packs and Mixer appearance. **Mixer style** offers Original, Clean Modern and Analog when the editable theme view is active. The Mixer layout menu separately changes which controls and panels are visible.
Use **Playlist clip brightness** under Theme > Options if the clip bodies are hard to read. Show this slider in Settings > Visibility if it is missing. It adjusts the current theme's display, with a separate value for the Vintage workspace.
Use the Theme Editor to preview changes and apply or cancel them. Export the theme if you want to share or reuse it on another installation.

### Audio device and latency
Keywords: settings audio normal advanced driver output device mode sample rate hz buffer length latency show asio panel reset underruns auto close
Open Settings > Audio. Normal shows Output device and Latency. Use a shorter buffer when playing live and a larger buffer if playback clicks or drops out. Advanced exposes the device's sample rate (Hz) and buffer length. Under ASIO, Show ASIO panel opens the driver's own controls in both Normal and Advanced, including when latency presets are available. Shared or Exclusive mode is shown for supported Windows and Linux devices.
Use the status and underrun counters to check that the selected settings took effect. Reset underruns clears the counter so you can check a new configuration. Available options depend on your driver and platform. Greyed controls show when a choice is unavailable.

### Why does audio stop when switching apps?
Keywords: switching windows Alt Tab focus background audio stops pauses no sound browser other apps auto close exclusive
Open Settings > Audio, switch to Advanced and expand **Input / output**. Turn **Auto close** off if you want ML Studio playback to continue while working in another application. It starts off by default.
With Auto close on, leaving ML Studio briefly pauses a playing transport and releases the audio output. Returning to ML Studio reopens it and resumes playback. On Windows, switching to another ML Studio window keeps the app in focus. If the problem is that another app cannot use the same device, check **Shared** or **Exclusive** under Audio Mode on devices that offer the choice.
Related: Audio device and latency | Fix delay, crackles or no sound

### Audio processing and recording settings
Keywords: settings cpu generator mixer audio worker limit priority smart disable pdc resampling metronome preview recording format recording offset
Under Settings > Audio > Advanced > **CPU**, **Multithreaded generator processing** distributes eligible instrument work and **Multithreaded mixer processing** distributes independent mixer work. **Audio worker limit** accepts Automatic or 1, 2, 4, 6 or 8 workers. **Audio processing priority** offers Normal, High or Realtime on supported platforms.
Under **Mixer**, **Plugin delay compensation during playback** aligns paths using latency reported by plugins. **Playback resampling quality** affects live clips and built-in samplers, from 2-point linear to 512-point sinc. The default is 16-point sinc. Larger filters cost more CPU. Export has its own resampling setting.
**Browser preview track** and **Metronome track** route each signal to a Mixer insert. A value of 0, displayed as a dash, uses Master. Check a selected insert's mute and output route if samples preview silently or the click is missing. **Play truncated notes on transport** retriggers the remainder of sustained notes when you start or seek into them. **Reset plugins on transport** resets hosted plugins at those starts and cuts existing effects tails.
Recording offers 32-bit float, 24-bit PCM, or 16-bit PCM. Recording offset moves recorded audio on the timeline. Positive values place it later. Measure against a reference before compensating for a hardware round trip.

### Save CPU with Smart disable
Keywords: smart disable hosted plugin idle suspend cpu silence VST2 VST3 tails sequencer sleeps modulation audio engine
Under Settings > Audio > Advanced > CPU, enable **Smart disable** to let eligible VST2 and VST3 instances sleep during silence. It starts off. Show the option in Settings > Visibility if it is missing.
Eligible instances sleep after four seconds of silence plus any reported finite tail and latency. An effect must also have silent sidechain input. Notes, MIDI, automation, parameter edits and transport changes wake the plugin. Export always processes plugins fully.
Open or busy editors, held voices, auxiliary-output instruments, ARA playback, and plugins with unknown or infinite tails continue processing. If a plugin has an internal sequencer or modulation that behaves differently after an idle gap, turn Smart disable off and compare playback.
Related: Audio processing and recording settings | Read signal levels, latency and plugin load

### MIDI devices, controller type and sync
Keywords: settings midi input output external sync ports master sync offsets curve velocity release pickup mcu generic transport learn monitor pedal
Enable the devices you use under MIDI > Input or Output. Match ports for plugins that listen to a particular port. Choose a Controller type per input. Use Generic MIDI for keys and the DAW port in MCU mode for compatible mixer and transport hardware. Use MIDI Monitor or Refresh device list to verify input. DAW MIDI Learn assigns actions to hardware buttons.
Velocity and release links and curves control how incoming dynamics are mapped. Pickup (takeover mode) helps avoid value jumps. Auto accept detected controller, Foot pedal controls note off, and Toggle on release adjust input handling. Omni preview MIDI channel, Song marker jump MIDI channel and Generator muting MIDI channel assign channels to those functions.
For ML Studio to lead, select the output and enable **Send master sync**. Under **Synchronization type**, choose MIDI clock for tempo-based sync or the MTC frame rate that matches the receiving device. Use **Master sync offset** to adjust timing. To follow another device, use **External sync** and its own offset, then check the status before recording.

### Use MIDI velocity curves, pickup and special channels
Keywords: MIDI pickup takeover controller knob jumps does nothing velocity curve sustain pedal release omni preview marker mute
Open Settings > MIDI > Input. **Pickup (takeover mode)** waits until a linked knob or fader reaches or crosses the current software value before it takes control. If a linked hardware knob appears unresponsive, sweep it through the software value. **Auto accept detected controller** closes the link dialog after the first controller movement it detects. Disable it to inspect the assignment before accepting.
**Link note on velocity to** routes the mapped attack velocity to Velocity, Mod X or Mod Y, or disables the link. Use the velocity curve to adjust the response to playing strength. **Link release velocity to** controls whether the release value is used. **Foot pedal controls note off** makes CC64 sustain released notes until the pedal comes up. With the setting off, CC64 goes to the instrument.
Assign separate MIDI channels to **Omni preview MIDI channel**, **Song marker jump MIDI channel** or **Generator muting MIDI channel** to use their white keys for those functions. A dash leaves the function unassigned. **Toggle on release** gives the muting keys a temporary action while held. Use a different input channel for normally played notes if they are triggering one of these actions.
Related: Map a MIDI knob or controller | Use MIDI output, sync and the MIDI Monitor

### General display and language
Keywords: settings general display ppi scaling high visibility smoothing transparent animations colorful waveforms thick lines force refreshes language note names black notes
Under Settings > General > Display, **Main GUI scaling** sets the workspace size. System follows the monitor's operating-system scale. **Pop-ups scaling** and **Toolbars scaling** each have a Main choice that follows the workspace or a separate percentage. The display PPI ruler lets you calibrate the displayed inch against a physical ruler.
**Animations** controls animated routing cables. **Smoothing** eases editor scrolling. Its lowest value makes scrolling jump directly to its destination. **Animation refresh rate** caps animated redraws at 40 fps in Less smooth, 60 fps in Smooth, or the display refresh rate in Ultrasmooth. **High visibility** brightens selected controls, including scrollbars and Channel Rack labels. **Colorful waveforms** changes waveform display.
This build stores **Transparent windows**, **Transparent menus**, **Force refreshes** and **Thick lines** as preferences. These currently have no visible effect. Thick lines is greyed out below 200% effective UI scale.
Under General > Language, **Interface language** offers English, French, Spanish, German, Brazilian Portuguese and Italian. Main-menu commands, static submenus, more General/Audio/MIDI/plugin settings, and Open/Save controls use the selected language immediately. File paths, project names, plugin names and user presets remain unchanged. Coverage still varies in specialized editors and third-party plugins can use their own language. Note names and Black notes independently choose English, Germanic or solfege notation and sharps or flats.

### General input and Undo
Keywords: settings general input mouse optimize for double click reset hover menus piano zoom keys undo warnings max levels
Under Settings > General > Input, **Manage shortcuts** opens Controls. **Piano Roll zoom keys** picks Page Up/Down alone or with Ctrl, Alt or Shift. **Vertical scrolling speed** and **Horizontal scrolling speed** adjust scrolling independently in each direction. **Double-click resets faders and knobs** is off by default and applies to supported ML Studio controls when enabled. Use Workspace Visibility to show this switch or **Open menus on hover** if either is hidden.
With **Use both mouse buttons in step sequencer** on, a left click sets a step and a right click clears it. With it off, either click toggles the step. In both modes, dragging over steps sets them with the left button or clears them with the right button. **Click-and-hold & special gesture functions** enables cloning by holding a note or clip and cycling the note type when holding a newly drawn Piano Roll note. **Optimize for** and **Invert pencil buttons** currently retain their values without changing pen or touch handling.
Under **Undo history**, **Maximum undo levels** keeps 1–999 project edits, with a default of 100. **Undo knob tweaks** includes supported knob changes. **Alternate undo mode** makes repeated Ctrl+Z step back and Ctrl+Alt+Z redo. Turn it off to use Ctrl+Z to toggle the last edit and Ctrl+Alt+Z to step further back. Check Edit for the active bindings.
**Show undo history in the browser when undoing changes** opens Current project > History when you undo. **Show recent changes at the top** reverses the list. **Put undone recordings in the recycle bin** is stored but currently has no effect.
Under **Hints and warnings**, use **Manage messages** to bring back suppressed confirmations. **Project data size warning** sets the threshold for a message after saving, from 50 MB through 1.5 GB, or Don't warn me. The default is 100 MB. System file association depends on your platform. The **Automatically check for updates** and **Automatically download program updates** checkboxes in General are unavailable in this build. Use the update control in **Help > About**.
Related: Open menus by hovering | Customize mouse and keyboard behavior | Undo and redo

### General startup and editor defaults
Keywords: settings general miscellaneous advanced startup default template plugin windows focus meters sample read loading
Under Settings > General > Miscellaneous, **Startup project** chooses Empty project, Default template or Last used project when launching ML Studio directly. **Default template** selects the template for new projects. **Show startup splash screen** controls the startup logo. **Silent startup** mutes the Windows startup chime.
**Auto name channels** names an existing channel after a sample or channel preset loaded into it. **Auto name effect slots** names a slot after a loaded effect preset. **Auto-route Channel Rack samples to mixer inserts** routes new Rack samples to available inserts. **Auto zip empty channels** folds channels without notes on a pattern change. **Select first channel with notes when the pattern changes** changes which channel is selected at the same time.
**Auto select linked modules** selects the assigned Mixer track when you focus a linked plugin. **Auto-scroll mixer while routing** helps reach destinations beyond the current view. **Restore previous state after solo** restores the mute states from before you soloed a channel, track or insert. With the setting off, leaving solo turns channels back on. **Alternate meter scale** changes how Mixer levels are displayed near 0 dB.
**Auto zoom in piano roll** fits the active pattern and channel notes when opening the editor. **Small scrollbars in editors** uses thin scrollbars without arrow buttons. **Monophonic painting in piano roll** removes notes at other pitches under the note you paint. **Create automation clips at the playback position** starts new clips at the playhead when there is no selected time range. Turn it off to start them at bar 1. A time selection takes precedence.
Related: Choose where plugin editor windows open | Use sample metadata and loading preferences | Native projects, folders and templates

### Choose where plugin editor windows open
Keywords: plugin window first launch default detached second monitor editors limit plugin toolbar off screen keyboard
Go to Settings > General > Miscellaneous. **Detach all new plugins** changes the default for plugins added from now on. Existing editors keep their saved mode. On Windows, **Open detached plugins with the main window** puts a newly detached window on the main window's monitor. An editor with a saved position reopens there. A plugin window's D button changes the current editor's mode.
**Limit the number of open plugin editors** closes the earliest opened editor when opening another would exceed 25. Closing an editor keeps audio processing active. Under General > Advanced, **Don't limit windows to screen** lets you drag a detached plugin title bar past the screen edges on Windows. Docked windows stay constrained to the workspace. **Cursors scaling**, **Legacy scaling**, **Hide plugin window toolbar by default** and **Disable typing keyboard priority by default** currently store preferences without changing editor behavior.
Related: Put ML Studio windows on several monitors | Plugin manager and plugin editors

### Use sample metadata and loading preferences
Keywords: WAV acid smpl unity pitch C5 beats root key tempo time stretch resample RAM disk threaded loading project sample preview
Under Settings > General > Advanced, **Read sample root note** uses the WAV's embedded root note when loading it into a Sampler. If the file has no root tag, the loaded sample defaults to C5. Turn the option off to keep the sampler's current root. **Read sample tempo information** uses an embedded WAV beat count to select Auto stretch and fit the sample to those beats. With no beat count or with the option off, existing stretch settings stay in place.
**Auto keep long audio on disk** enables Keep on disk when a file of at least 30 seconds is loaded into a Sampler. Existing channel switches keep their values. **Threaded project sample loading** allows up to four workers to decode samples when opening a project. Turn it off to load one file at a time. Browser previews already begin when the start of the file is decoded. **Fast sample preview** currently stores a preference without changing that behavior.
**Initialize controls automatically** records the current value as the project's initial value when you create an automation clip. This replaces an earlier initial value for the same control in the same Undo step. Disable the preference to keep initialization manual.
Related: Play and shape samples in ML Sampler | Record automation and manage initial values | Fit a loop or use its detected tempo

### File, backups and sample libraries
Keywords: settings file autosave location maximum user data folder factory folder browser extra search folders repair optional features reset browser cache
Settings > File > **Backup** sets **Autosave** to Never (with a reminder), every 15, 10, 5 or 1 minute, and offers modes that also save before eligible risky operations. The default is every ten minutes. **Location** chooses the global folder or the project's Backup subfolder when it has a data folder. **Maximum** retains 1–999 backups per project name, defaulting to 100. The next backup removes older entries beyond the limit.
Open the Browser's Backup area to find earlier revisions. After confirming a recovered revision, save it under a separate name to keep the previous saved project available. Use File > Save deliberately when preparing a copy to share.
File shows the User data folder and the factory content location. Add or remove Browser extra search folders to find sample libraries. Changing a search root changes where ML Studio looks for files. File > Troubleshooting offers Repair optional features and Reset browser cache.

### Plugin manager and plugin editors
Keywords: settings plugins formats search folder find new plugins rescan all rescan plugins with errors combine formats prefer vst clap hide favorite
In Settings > Plugins, enable the folders you want scanned. Add a folder for an installation outside the standard locations. Find new plugins scans additions and Rescan all rebuilds scan results. Use Rescan plugins with errors when retrying failed scans.
Combine formats of the same plugin and the format preference affect which variants appear in the picker. Use the table to filter, favorite, or hide plugins. Use the plugin window's D control to detach or attach its editor. Projects keep the plugin identity and still need a compatible installation on each computer.

### Choose where a project keeps its audio
Keywords: data folder relative absolute specified path copy used samples missing external backing up move portable project samples recordings
Open Settings > Project > **Data folder**. Choose **Store data relative to the project file** to keep project data alongside the saved project, or **Store data in the specified folder** to pick a dedicated location using **Set data folder...**. **Don't use a data folder** leaves the project without a dedicated folder. Review the prompt when changing locations and save the project after choosing.
**Ask for data folder** controls when the new/save form prompts, such as on startup, on creating a project, or on the first save. **Copy used samples to the project folder** can collect the current samples into its Samples subfolder and copy later sample imports there. If you skip copying existing samples, their previous paths still matter. Use a zipped project when you need a transfer that includes its audio.
Related: Project data, time and credits | Send a project with its audio files | Save, reopen and recover a project

### Customize Traditional Mixer insert knobs
Keywords: theme traditional FL style mixer plugin insert slot mix dry wet knob ring outline circle color full mixer
Open Settings > Theme > Traditional FL-style mixer view > Customize insert knobs. Choose **Full Mixer** to use the same knob-cap artwork as the Full Mixer finish, including Mixer Skin Designer artwork. Traditional, Dial and Ring are also available. Inherit restores the theme's original choice.
The **Thin value ring** shows the wet/dry amount. **Outer circle** is the separate surround. Show or hide each independently, choose each color, and set widths from 0 to 2 pixels. The live examples show 0%, 50% and 100%. These settings affect Traditional insert slots; routing, wet/dry values, automation and Full Mixer styling stay intact.
Use Apply theme to keep your edits, Cancel edits to restore the starting appearance, and Undo/Redo while experimenting. Reset section restores this area's controls together. Save a preset or Export theme to reuse the look.
Settings > Visibility > Theme customization can hide the Traditional insert knob settings without erasing their saved appearance.
Related: Themes, icons, cursor and mixer design

### Grade colors across the workspace
Keywords: theme saturation brightness contrast hue colors all workspace playlist piano roll browser menus rack mixer
Open Settings > Theme > Open Theme Editor. Expand an area or search for it. There are separate controls for Toolbar & shared controls, Browser, Window frames, Playlist, Mixer, Attached mixer inspector, Channel Rack, Menus & dialogs, Piano Roll, Control finishes, Piano Roll note groups, Workspace, Shared dial controls and Traditional FL-style mixer view.
Each area's palette has hue, saturation, brightness and contrast. Hue 180 and the other values at 100 leave that area unchanged. Saturation 0 makes its palette grayscale; values above 100 increase saturation. Adjustments tint custom colors in the same area too. The original values stay in the theme so Reset and Undo can restore them. The main Theme page's Adjustments remain the controls for the overall theme.
Shared dial controls now include the unfilled track and outer-border colors, independent visibility for the value ring, track and border, and ring/pointer widths. These settings select the shared themed dial renderer. Full Mixer authored finishes keep their own designer controls.
Apply, Cancel, Undo/Redo, preset saving and portable theme export include these settings. Reset section restores its colors, shapes and area adjustments together. Workspace Visibility > Theme customization > Area color adjustments hides those controls without changing the theme.
Area adjustments operate on the app's named color palette. Fixed artwork, the Vintage appearance overlay and third-party plugin editors retain their own designs; turn Vintage off to use editable themes. No project colors, plugin parameters or audio are rewritten. Color grading happens when the theme changes, not during playback or each repaint.
Related: Customize Traditional Mixer insert knobs

### Themes, icons, cursor and mixer design
Keywords: settings theme appearance skins adjustments theme editor mixer skin designer icons fonts cursor packs note colors brightness
Settings > Theme lets you choose a skin and theme and adjust colors and brightness. Audio and automation clips use note colors changes how clip colors are derived. The Playlist scrollbar style is also adjustable when its visibility option is enabled.
Open Theme Editor for individual color and shape settings or Mixer Skin Designer for mixer-specific appearance. Editing a mixer body, panel, slot or fader color selects the editable Theme colors finish automatically; the Traditional and Full layouts both use those colors. Editing a skin component color enables that custom component. Apply keeps the changes, and Cancel restores the previous appearance. Icons & Fonts lets you choose icon and cursor packs independently, override individual glyphs, and change the interface font. Save a preset or export the theme to reuse the appearance.

### Project data, time and credits
Keywords: settings project data folder ask copy used samples ppq timebase signature division panning law crossfades info debug about licence
Project > Data folder controls where project audio is stored. Choose when to ask for a data folder and whether to Copy used samples to the project folder. Time settings sets Timebase (PPQ) and a time signature or time division. Changing the signature changes the bar grid. Use the tempo control to change BPM.
Project > Advanced includes Panning law and Play truncated notes in clips. Playlist contains Automatic crossfades and the default for new projects. The Info tab saves the title, author, genre, comments, web link, and Show info on opening. It also shows working time, with a reset control.
About shows version, licence and product links. Debug can be enabled in Workspace Visibility for diagnostics. Debug details can help when describing a reproducible problem.

### Change the time signature, steps per beat or PPQ
Keywords: meter grid 3/4 6/8 beats per bar steps per beat numerator denominator timebase PPQ ticks panning law automatic crossfades
Open Settings > Project > **Time settings**. Choose **Set as time signature**, then set **Numerator** to the beats per bar and **Denominator** to the note value of a beat. The numerator offers 1–16 and the denominator offers 2, 4, 8 and 16. To set the step grid directly, choose **Set as time division** and set **Beats per bar** (1–32) and **Steps per beat** (1–16).
**Timebase (PPQ)** sets timing resolution in pulses per quarter note. Changing it rescales stored positions to the new resolution. Check fine note and clip edits after lowering PPQ. For a faster or slower song, change the toolbar BPM. To work at a different visible grid interval, change snap in the editor.
Under Project > **Advanced**, **Panning law** selects Circular or Triangular. It affects how panned audio levels vary across stereo. Under Project > **Playlist**, **Automatic crossfades** applies to the current project. **Enable automatic crossfades in new projects** supplies the default for future projects.
Related: Project data, time and credits | Set tempo and play the right thing | Import, trim and fade audio

### Workspace visibility, menus and controls
Keywords: settings visibility choose what to show menus customize controls bindings portable action shortcuts theme toolbar
Visibility lists the current show/hide controls by workspace area. Search for an exact label to find its switch. Open it directly from Settings > Visibility or through General > Workspace. The settings apply per profile, and Restore default visibility restores the shipped choices.
Menus lets you organize menu entries and restore their structure. Controls manages keyboard shortcuts, mouse bindings, action sequences, and toolbar buttons. Back up your controls with Export and import a supported controls file or portable action script when moving profiles.

### Put ML Studio windows on several monitors
Keywords: dock detach D second monitor external screen detached guide browser mixer playlist piano roll plugin windows layout
Use a window's Detach command or D control where offered, then move its native title bar to the second monitor. Use Dock or D again to return it to the workspace. For the Browser, use its menu. For Help > User Guide, click Detach or Dock at the top of the guide.
The guide's separate window keeps its own article, query and scroll. The main workspace can keep editing and playing while the guide is open. Use the native close button to hide it and Help > User Guide to reopen it. Window recovery uses the available monitors to bring a saved frame back into view if a display is removed.
Project and named workspace layouts cover the supported editors. Use the guide's own controls for its position and docking mode.
Related: Browse or search the User Guide | Show a window or restore your workspace

### Arrange the toolbar and utility windows
Keywords: toolbar presets panels detached tap tempo touch controller plugin performance project picker
Use View > Toolbars to choose visible panels and access toolbar layout editing. Move or hide sections you rarely use and use Workspace Visibility to expose optional buttons. Save a layout if you switch between recording, arrangement and mixing setups.
Open the Tempo tapper to estimate BPM from taps, the Touch controller for on-screen control, and Plugin performance monitor to inspect processing load. The Project Picker helps locate project patterns, clips and other sources. Plugin Picker opens the installed instrument/effect catalog.
Related: Show or hide interface features | Build action sequences and toolbar buttons | Read the toolbar and project time

## Files, export & sharing
Overview: Move projects, export audio, and collaborate.

### Save, reopen and recover a project
Keywords: lmn flp backup save as autosave portable missing files
Use File > Save to keep working or Save As to make a separate version. Keep the project's audio files and installed plugins available when reopening it.
Use the File menu's ZIP project export when you need a portable project and its audio. Third-party plugins must be installed on the destination machine.
If a project crashes or a save is lost, check the available backups before overwriting the original project.

### Import a project from FL Studio
Keywords: flp compatibility import report missing sample plugin migrate
Open the FLP through File > Open. Review the Project Import Report if it lists missing files or unresolved links, and point the project to the original samples where needed.
Install compatible plugin formats before opening a project that uses third-party plugins. Check playback, automation, routing and settings against the original, then save a separate working copy.

### Export a finished song, stems or MIDI
Keywords: export render wav mp3 stems multitrack midi
Related: Send a project with its audio files | Freeze or bounce a track
Guide: export
Choose Song mode and confirm the arrangement. Use File > Export audio to open the export form.
Export audio opens in Easy mode with 24-bit WAV and 320 kbps MP3 options. Pick either or both. Easy uses stereo and the standard render settings. The Advanced tab has additional formats, source range, quality, effects, and split mixer tracks. Advanced export choices save separately.
Use WAV for a full-quality mix, MP3 when a smaller listening copy is useful, and MIDI for notes and timing. Use split Mixer tracks when you need separate audio stems.
Listen to the exported file and verify its start, end, levels and effects before sending it.

### Send a project with its audio files
Keywords: share project pack zipped audio samples collaborate
Guide: export
Use File's ZIP project export to gather the project and its referenced audio files. Open a copy of the archive to check that it contains what the recipient needs.
Tell the recipient which third-party plugins and formats the project uses. For a collaborator without those plugins, include audio renders of important tracks.

### Collaborate on a project
Keywords: live collaboration host join invite network transport follow recovery
Choose File > Collaborate. Save the project, stop playback, and host on a reachable address. Share the invitation privately. Your collaborator opens the same dialog and joins with that invitation. Both users need compatible ML Studio builds and their own copies of the plugins used in the project.
Reopen Collaborate for session status, playback following, conflict resolution, recovery files, and End or Leave. Each computer renders its own audio. Use a LAN or shared private VPN for direct connections.

### Native projects, folders and templates
Keywords: lmn flp extension file format project data folder copy used samples save new version template delete unused
New saves, backups and ZIP projects use .lmn. File > Open also accepts compatible .flp files and zipped projects. Save on an existing .flp keeps that name. Use Save As to make a separate .lmn copy before editing a migrated project.
Use Save new version for a numbered sibling or Save to new project folder to gather the session in a different folder. Under Settings > Project, choose the project data location and whether to copy used samples. Recording and render folders are part of a portable audio project.
Use Save as template and select it from File > New from template or set it as the General default. Revert to last backup when that is the desired recovery point. Delete unused files and save project is a project cleanup action. Read its confirmation and keep wanted source and archived versions before running it.
Related: Save, reopen and recover a project | Import a project from FL Studio | Send a project with its audio files

### Import and export MIDI files
Keywords: standard MIDI SMF import tempo map channel track piano roll export midi notes
Choose File > Import > MIDI file and review how tracks, channels and tempo are imported. Use Piano Roll's import when the file should populate that editor's target. Drop a MIDI file into the Playlist when you want a new pattern at the drop position.
Use File > Export MIDI for the supported note and timing data. To transfer instrument sounds, mixer processing and audio clips, also export audio. Reopen the MIDI file in the receiving app to check tempo, channel assignment and note lengths.
Related: Edit several MIDI clips on one timeline | Export a finished song, stems or MIDI

### Export settings, metadata and stems
Keywords: WAV FLAC MP3 OGG dither stereo mono gapless HQ plugins tail loop points markers tempo resampling split mixer tracks
In File > Export audio, use Easy for 24-bit WAV or 320 kbps MP3. In Advanced choose WAV, FLAC, MP3 and OGG as needed. Set format depth or bitrate, sample rate, stereo or mono and interpolation quality. Check the output path and filename before exporting.
Choose Song or Pattern and Whole or Selection where available. Tail can cut, ring out, or wrap back to the start for loops. Insert effects and Master effects are independently enabled. HQ for plugins asks hosted processors for offline high-quality mode where supported. Dither is available for integer formats that support it.
Enable Split mixer tracks for WAV or FLAC stems and choose whether to include Master. WAV metadata options can write tempo, Playlist markers, loop points and pattern note markers. Gapless MP3 metadata helps compatible players handle encoder delay. Verify start, end and levels in the final files.
Related: Export a finished song, stems or MIDI | Freeze or bounce a track

### Export a seamless loop or keep effect tails
Keywords: export render tail cut ring out wrap loop seamless reverb delay selected range pattern gapless encoder
Open File > **Export audio** and choose **Song** for the Playlist or **Pattern** for a pass of the current pattern. Use **Selection** when you have set the intended time range in the relevant editor. Verify the range before rendering.
Use **Tail > Cut** when the file must stop at the end of the selected material. **Ring out** includes the decay after that point. **Wrap** puts the sound that would extend past the end back at the beginning and keeps the original length. For a loop, audition the finished file across its join in the player where you will use it.
For MP3, **Gapless** writes encoder delay information that compatible players can use. Some DAWs still play extra silence. WAV or FLAC avoids the MP3 encoder delay for precise reimport.
Related: Export a finished song, stems or MIDI | Export settings, metadata and stems

### Export mixer stems and choose what gets processed
Keywords: export stems split mixer tracks master insert effects hq bit depth dither resampling render multitrack individual stems
Open File > **Export audio** and show Advanced options. If the tab is missing, show **Settings > Visibility > Workspace > Advanced Export tab**. Choose **WAV** or **FLAC** and enable **Split mixer tracks** to export the Mixer paths as separate files in a stems folder. Enable **Include the master** if you also want the Master file with the split tracks.
**Insert effects** and **Master effects** control processing at those stages. **HQ for plugins** tells compatible hosted plugins to use their offline processing mode. Choose sample rate, channel format, bit depth and resampling quality for the delivered files. **Dither** adds triangular dither for 16-bit and 24-bit output. **Open the folder when done** reveals the result in the file manager.
Related: Export a finished song, stems or MIDI | Export settings, metadata and stems | Correct headphone monitoring

### Resolve collaboration conflicts and recover sessions
Keywords: network LAN TCP 32190 host guest invite TLS project shared conflict recovery disconnect local copy follow playback
Hosts and guests need compatible builds. Host after saving and stopping playback. Share the invitation privately, since it grants access to download and edit the project. The guest gets a fresh local project and sample folder. Each computer supplies its own compatible plugins and licenses.
Play independently or turn on Follow host playback for transport and position following. Audio is rendered on each computer. Remote structural or plugin-state edits may wait for local gestures, dialogs or playback to finish.
If the same item changes on both machines, open Collaborate and choose Keep my version or Use shared version. A local Before conflict copy is saved before resolving. Use Open recovery folder to find checkpoints and recovered work after a disconnect. End / leave keeps local projects. If hosting across networks, use a reachable LAN or private VPN address, or configure the host's firewall and port forwarding.
Related: Collaborate on a project | Send a project with its audio files

## Menus
Overview: Find commands in the main menus, editor menus, and right-click menus.

### How menus work
Keywords: menu dropdown popup context right click checkbox radio disabled arrow submenu shortcut customize keyboard mouse hover
Open the main menus at the top of the workspace. The small menu button inside an editor opens that editor's settings and tools. Right-click an object such as a clip, track header, channel, effect slot or knob to act on that specific object. Some tools use right-click for editing or erasing in the grid. Use the object's title, menu button or designated control to open its menu.
- A right arrow opens a submenu. Move onto the row and into its submenu to reach the command. A check mark shows an on/off choice. A radio mark shows the selected member of a mutually exclusive group.
- A greyed command needs a different selection or project state, or is unavailable in this build. For example, a project must be saved before it can revert to its own backup. Audio, pattern and automation clips offer different commands.
- The key printed beside a command is its shortcut in the relevant context. Settings > Controls can change the effective binding. When Typing keyboard to piano is active, many bare letter keys play notes.
- A menu's wheel moves among available rows. A menu that spans several columns can scroll sideways to reveal more items. Click outside or press Escape to close without choosing a command. With General > Input > Open menus on hover enabled, click a main menu to start. Hover over other main-menu titles or submenu arrows to open them, and click a command to run it. The same submenu behavior applies to context menus, Settings menus and detached ML Studio views once their menu is open. Press Escape or click outside to end menu navigation.
Related: Open menus by hovering | Main menu command index | Customize menus | Keyboard and mouse shortcuts

### File menu
Keywords: File new recent template save as version import MIDI export stems ZIP project cleanup backup collaborate
New starts from the default template. New from template lists available factory and user templates. Open accepts a project or supported ZIP, and recent projects appear in the File menu once you have opened them. More shows older entries.
- Save updates the current project. Save As creates a separate project name or location. Save new version creates a new numbered .lmn. Save to new project folder changes the project folder, while Save as template makes a reusable starting point.
- Import contains MIDI file, Project import report and Resolve imported files. Open is also the entry point for compatible FLP projects. Use the report and resolver to review what needs attention.
- Export audio opens the Easy/Advanced render form. Export MP3 goes directly to an MP3 export setup, Export MIDI writes musical data, and Export zipped project gathers the project and referenced media. Split mixer tracks is in Advanced audio export.
- Revert to last backup reopens the saved recovery revision. Delete unused files and save project cleans project data after confirmation. Use Collaborate to host or join a shared session. Exit uses the normal save/discard flow.
<!-- menu-reference File -->
Related: Native projects, folders and templates | Export settings, metadata and stems | Collaborate on a project

### Edit menu
Keywords: Edit undo redo history last action project snapshot alternate undo
Undo reverses the most recent project edit it can restore. The label can identify the action. Redo restores an undone edit where the current history and undo mode allow it. A new edit can change the redo path.
Choose Undo history to see recent project edits in the Browser. General settings controls history depth, knob tweak history, alternate undo behavior and history presentation. Audio Timing, Vocal Tuning and some plugin editors also provide their own local history controls. Check the active editor before undoing a detailed clip or plugin change.
<!-- menu-reference Edit -->
Related: Undo and redo | General input and Undo

### Add menu
Keywords: Add generator instrument effects Sampler Layer audio clip automation clip More plugins picker category vendor
Use the instrument entries to add a Channel Rack generator. Sampler creates a sample-playing channel. Audio clip and Automation clip create their respective sources. Layer is a channel for layered instrument triggering. Pattern opens a new pattern workflow.
Use Effect to select an effect for the Mixer. Choose the destination insert first. Plugin picker or More plugins opens a broader choice of installed instruments or effects. Use Categories, Simple or By vendor to change how plugin lists are grouped. Installed names and favorites depend on your plugin database.
Refresh plugin list performs the fast discovery path. Manage plugins opens scanning, paths and plugin filters. Use the regular plugin scanner after installing a plugin that does not appear in the picker.
<!-- menu-reference Add -->
Related: Add, replace and route an instrument | Find installed plugins | Insert effects, wet/dry mix and plugin bypass

### Patterns menu
Keywords: Patterns list select empty F4 find rename color time signature transpose clone insert delete move split render
The pattern list selects the active pattern. Find first empty and Find next empty create or select available pattern slots. The no-naming version skips the naming prompt. Use Rename and color, Change color or Random color to distinguish parts.
Select in playlist locates occurrences in the active arrangement. Open in project browser locates it in the Browser. Set time signature and Transpose act on the chosen pattern. Insert, Clone, Delete, Move up and Move down manage the pattern list.
Split by channel separates a multi-channel part. Quick render as audio clip uses the quick render workflow. Render as audio clip opens options. Render and replace substitutes the audio in the arrangement.
<!-- menu-reference Patterns -->
Related: Manage and render patterns | Program steps and patterns | Build an arrangement

### View menu
Keywords: View windows browser picker toolbars layout Arrange windows Into workspace detach close all tempo tapper touch controller
Show Playlist, Piano Roll, Channel Rack, Mixer and Browser from View or their toolbar shortcuts. Project Picker locates project material and Plugin Picker lists installed plugins. Tempo tapper and Touch controller are separate utility windows. Plugin performance monitor helps identify processing load.
Toolbars contains panel and layout controls. Arrange windows provides named workspace presets, Undo last layout change, and layout hotkey assignments. Into workspace brings editor positions back into the available workspace. Background controls the workspace appearance where available.
Close all windows closes editor windows, Close all plugin windows closes plugin editors, and Close all unfocused windows keeps the active editor. Browser-related entries jump to History, Patterns, Generators, Effects, Remote control and Plugin database where offered.
<!-- menu-reference View -->
Related: Show a window or restore your workspace | Put ML Studio windows on several monitors | Arrange the toolbar and utility windows

### Options menu
Keywords: Options audio MIDI general file theme plugins project info settings metronome precount snap follow playhead CPU meter
Use the top entries to open MIDI, Audio, General, File, Theme, Plugin Manager and project settings. Project info is the credits and working-time page. The MIDI switches control remote input, MIDI output and master sync.
The switches below them set Typing keyboard to piano, Metronome, Recording precount, Start on input, Blend recorded notes, Step Edit, Loop record, Follow playhead, Enable groups, and Multilink to controllers. Right-click supported toolbar switches such as Recording precount and Typing keyboard to piano for additional choices.
CPU meter chooses a Compact, Classic FL-style or Lemon live monitor display. The AI settings entry, where visible, opens the external MCP setup. Controls, Workspace Visibility and menu customization are available from Settings.
<!-- menu-reference Options -->
Related: Choose a recording mode and lead-in | MIDI devices, controller type and sync | Connect an external assistant with MCP

### Tools menu
Keywords: Tools macros one-click audio recording score logger last tweaked parameters stop sound cancel recording unused samples declicking
One-click audio recording offers supported capture destinations. Dump score log to selected pattern recovers recently played notes within its chosen time window. Clear log clears the note log. Last tweaked and Next to last tweaked address recent parameters and offer supported automation, event or controller actions.
Stop sound stops transport and releases active notes and previews. Cancel recording discards the active take. Select unused channels highlights candidates for cleanup. Purge unused audio clips removes unused clip sources and Reset empty Playlist tracks clears metadata from eligible empty rows.
Switch all or selected audio clip channels to changes their sample playback mode. Declicking controls how sample edges are treated. Switch all plugin editors changes supported attached/detached editor states. Remove all audio editor instances acts on Tesla slots, and the cleanup command for unused files uses the project's saved data folder.
<!-- menu-reference Tools -->
Related: Recover a played phrase with the score log | Record, edit and save audio in ML Tesla | Native projects, folders and templates

### Help menu
Keywords: Help User Guide Change Log About build Community Discord support diagnostics menu
User Guide opens this wiki. Use Contents, search, related articles and the Detach button to keep a reference beside your project. Change Log opens the bundled release history and has its own search.
About opens Settings > About, where you can find the build and license information. Community Discord opens the community link. Use Report a Bug in the app when you want to submit reproducible steps and choose which diagnostics or attachments to share.
<!-- menu-reference Help -->
Related: Browse or search the User Guide | Report a problem and check for updates

### Playlist menu and timeline menus
![playlist-menu | Playlist menu with the Edit submenu open](assets/guide/playlist-menu.jpg)
Keywords: playlist triangle options edit tools snap grouping zoom markers visibility arrangement track header ruler
The Playlist's menu offers Edit, Tools, Tool, Snap, Select, Zoom and Time markers. Use Edit for clipboard actions, Duplicate, Make unique, space insertion/removal and clip mutes. Tools includes consolidation, Playlist track export and quick quantization. Tool selects Pencil, Brush, Comp, Slice, Select, Delete, Mute, Zoom, Slip or Playback.
View and visible clip controls determine which editing details are shown. Snap controls timeline placement. Select includes similar or muted sources and time selection around clips. Group and Ungroup change which clips move together. Zoom offers focused, in/out and selection-based navigation.
- Right-click a track header for rename and color, row height, folders and groups, move/insert/delete, mute and solo, track type and routing, take lanes, Freeze and Bounce. Some attached Mixer rows use Mixer ordering and routing.
- Right-click the ruler or a marker for markers, loop placement, time signature, marker behavior and moving content. Marker profiles can save, replace or merge markers. Open the arrangement name menu to switch or manage alternate arrangements.
- Use the Clips menu to show the Clip Picker and choose source visibility. Right-click a source for commands for its clip type.
Related: Select, group and move Playlist tracks | Arrange with groups, markers and zoom | Set up an alternate arrangement

### Audio, pattern and automation clip menus
Keywords: clip context menu audio processing stretch unique stem ARA vocal align pattern edit automation point tension
An audio clip's menu offers Preview, Muted, Rename, Make Unique, Mixer routing, Clip effects and color actions where applicable. Time / stretching includes Detect sample tempo, Make it fit, fit to bars, half/double length and reset duration. Trim clip edges and Stretch clip edges change what the edge drag does.
Use Audio processing for tools such as Strip Silence, Normalize audio clips, ARA editing, Stem separation and Tesla cleanup when available. Audio Timing and Vocal Align expose editing workflows for supported audio. Comping can take a selected range or the whole take. Select & organize includes selecting similar clips.
A pattern clip has Edit pattern, selected MIDI clip editing and pattern stretch actions where relevant. An automation clip works with its linked control and curve. Right-click a point to choose its interpolation Mode, copy or paste a value, type an exact value or delete it. The selected clip and source determine which items appear.
Related: Process clips, render edits and keep source audio | Edit several MIDI clips on one timeline | Edit curves and manage targets

### Piano Roll menus
![piano-menu | Piano Roll Tools menu and its keyboard shortcuts](assets/guide/piano-menu.jpg)
Keywords: Piano Roll menu tools selection edit snap note length color keyboard style control lane MIDI parts stamp ghost
Open the Piano Roll menu for File, Edit, Tools, Stamp, Select, Snap, View, Helpers, Time markers and MIDI parts. Use File to import or export MIDI in the current editor context. Edit and Selected notes offer clipboard operations, moving or copying notes to another channel, shifts, transposition, octave moves and note mute. Selection commands can select all, invert, or select by color.
Tools provides Quantize, Legato, Chop, Glue, Strum, Arpeggiate, Flam, Claw, Limit, Flip, Scale levels, LFO and Randomize where available. A tool may act on all notes when there is no selection. Check its preview or the selection before applying a change.
Use Snap, Note length, Note color and Draw mode for new notes. Slide/glide and portamento change supported note behavior. View and Helpers include keyboard style, note grid highlights, the miniature overview, the control lane and key/scale guidance. MIDI parts switches among clips in the shared timeline. Right-click or double-click the appropriate note or lane control for its own actions.
Related: Use the Piano Roll editing tools | Edit note properties and the event editor | Work with chord and scale tools

### Channel Rack menus
Keywords: Channel Rack options channel context graph filter selected sort fill transpose mixer number
Open the rack Options menu for Add instrument, Edit channels, Organize, Mixer routing, Audio tools and View. Edit channels handles selected channel operations, including clone, mute, delete, transpose and color. Organize handles groups, filters, sorting, moving and cleanup. Mixer routing assigns or selects inserts. Audio tools provides dry consolidation and wet bounce where available.
Right-click a channel name to open Piano Roll, change or insert an instrument, open the graph editor, load a sample, change sample tempo or stretch, set Cut itself, edit pattern steps, consolidate, rename, clone or delete. Fill each 2, 4 or 8 steps gives an even pattern. Advanced fill and step rotation modify the current pattern.
Right-click the Mixer number for destination and selected routing options. Right-click a knob for its value and automation options. Use the LED context for channel mute and solo. Rack View controls the Mixer number display, zipped rows, name alignment, button width and automatic height.
Related: Select and edit several channels | Route selected channels to the Mixer | Program steps and patterns

### Mixer and effect slot menus
Keywords: mixer options track insert slot FX presets spare state scaling PDC dock to routing layout inspector
Open Mixer Options for the inspector, its location and layout, recording-to-Playlist behavior, automatic Plugin delay compensation, View, attachment to Playlist, Detached, unused tracks, Smart Disable and audio editor cleanup. Use the Mixer layout selector to choose Traditional, channel strip, Full and available Vintage views.
Right-click an insert for rename, color, track insertion, save effect chain, reset, solo, separator and channel layout. Change icon, Dock to and the track-menu Render to wave file entries are unimplemented placeholders in this build. Use the documented export/bounce workflows to render audio. The send controls provide routing actions. Track locks can limit changes on an insert. Choose the insert before opening a menu for its effect list or inputs.
Right-click an occupied effect slot for Replace, Delete, Browse parameters, Sidechain input, Move up/down, View editor and Rename. Preset/spare-state, per-slot CPU, scaling and several appearance entries in this particular slot menu are placeholders, not working commands. Use the plugin wrapper or its own editor for supported preset operations, and Settings > Audio for global Smart disable. An empty slot opens a different picker context.
Related: Save and reuse effect chains | Attach the Mixer to the Playlist | Create sends, buses and sidechains

### Browser, plugin wrapper and parameter menus
Keywords: browser settings file context favorites tags filter plugin wrapper presets spare state copy value type in automation controller
Open the Browser's arrow menu for content roots, tabs, preview behavior, Starter Pack, docking, appearance, search and folder navigation. Right-click a file or item for its available actions, such as locating the source folder, Favorites, tags, sending to a selected channel or Mixer, and supported filesystem actions. Remove from browser removes its browser entry where offered. Delete file acts on the file itself.
Use the plugin window's menu for presets, Save preset as, spare state, favorite/database actions, Browse parameters, threading or Smart Disable, name/color/icon and Attach/Detach. Instrument wrappers can offer Piano Roll, channel state and routing. Native editor menus are supplied by that plugin.
Right-click an automatable control for Reset, value entry/copy/paste, automation, events or Link to controller where supported. Reset returns a parameter to its default. Type in value is for precise editing. Choose Create automation clip for timeline control or Link to controller for a hardware binding.
Related: Browse samples, favorites and the Starter Pack | Load and save presets | Create an automation clip

### Reorganize or restore a menu
Keywords: Settings Menus customize hierarchy group submenu hide reorder move folder dissolve undo restore default layout
Open Settings > Menus and choose a menu. The selector includes the main menus and supported Playlist, Piano Roll, Channel Rack, Mixer and Browser menus. In editor menus with Customize menus, use it to jump to the matching editor.
Expand a submenu to see its children. Use the visibility control to hide a row, the reorder buttons to move it among siblings, and the move control to put it in an eligible submenu. Add a named submenu for commands you use together. Rename or dissolve a custom group when you change the structure. Hiding a parent also hides its children.
Use the menu editor's Undo to revert a menu change or restore the selected menu's default structure. Layouts are saved per profile. Use Workspace Visibility for feature-level switches and Controls for shortcut assignments.
Related: Customize menus | Show or hide interface features | Restore a shortcut that stopped working

## Reference & support
Overview: Look up commands, defaults, compatibility and fixes.

### Undo and redo
Keywords: history revert mistake ctrl z
Use Edit > Undo or Undo history to step back through project edits. Redo restores edits that were undone if you have not made a new conflicting edit.
Playlist pitch and timing editors offer separate Undo/Redo controls for the active clip. Use the project Undo and Redo shortcuts shown in Edit or your assigned Controls bindings for project history. Each completed gesture, reset, or clip-history action is one project undo step.

### Report a problem and check for updates
Keywords: report bug build update restart troubleshooting support
Use Report a Bug and include your build, operating system, exact steps, expected result and actual result. A small project or screenshot helps when it can be shared safely.
Use the update control to check for a signed release. Save your work before restarting to apply it. If the updater cannot start, ML Studio stays open and shows the error. If installation fails after closing, check the message when ML Studio reopens. You can also install a public update from the official downloads page. **Help > About** opens the page with the build number.

On Windows, clicking Update to restart can display a Windows permission prompt if the helper or installation folder requires administrator access. Approve it to continue, or cancel to keep working and retry later. A standard Windows account may need an administrator's credentials. The updater verifies the downloaded files and folder access before closing your project. Automatic installation during an ordinary exit defers permission requests until you explicitly choose Update. If Windows policy blocks the helper, the error remains visible for troubleshooting.

### Check for an update after a manual install
Keywords: updater update button manual install Windows error 740 permission admin signed restart download update
On the Windows public copy, open **Help > About** and check the installed build number. If you manually installed the newest release, the next update will become available when a newer compatible build is offered. Use the update control in About to download and apply it.
Windows may ask for permission when the helper starts or the destination requires administrator access. Approve the prompt to continue. If Windows reports error 740 on an older build, install the current signed public package, reopen ML Studio and verify the build in About. If Windows still blocks the updater, include the build and full error in your report.
Related: Report a problem and check for updates | Install and open ML Studio

### Main menu command index
Keywords: menu reference complete commands file edit add patterns view options tools help shortcuts
Use this index to look up commands by menu. Available items and key assignments can vary with the selected editor, project state, platform, and your Workspace Visibility and Controls settings. Menus also have contextual submenus for the active clip, channel or plugin.
<!-- menu-reference -->
Related: Keyboard and mouse shortcuts | Customize menus

### Keyboard and mouse shortcuts
Keywords: keyboard shortcut hotkey mouse function keys ctrl cmd alt shift wheel F5 F6 F7 F8 F9 F10 F11 F12
#### Start, save and switch windows
- Space plays or stops. Ctrl+Space plays or pauses. Use PAT or SONG to choose the source.
- Ctrl+O opens a project. Ctrl+S saves. Ctrl+Shift+S opens Save As. Ctrl+N saves a new version.
- F5 opens Playlist, F6 Channel Rack, F7 Piano Roll and F9 Mixer. F8 opens Plugin Picker by default. Ctrl+F8 opens Project Picker, Alt+F8 Browser and Shift+F8 Plugin database. F10 opens MIDI settings and F11 Project info.
- F12 closes editors. Alt+F12 closes plugin windows. Ctrl+F12 closes unfocused windows.
#### Arrange, edit and record
- F4 finds the next empty pattern. F2 names or colors the current pattern. Ctrl+Z undoes. See Edit for the active Redo behavior.
- Ctrl+T toggles Typing keyboard to piano. Ctrl+M toggles metronome. Ctrl+P toggles Recording precount. Ctrl+E toggles Step Edit.
- Ctrl+R opens audio export. Ctrl+Shift+R opens MP3 export. Ctrl+Shift+M opens MIDI export.
- Ctrl/Cmd+Shift-drag can marquee-select notes or clips. Ctrl-wheel zooms where supported. Use the editor's tool and snap menu to check the active behavior.
#### Search and customize
- In the User Guide, Ctrl+K or Ctrl+F focuses search. Enter opens the first result. Escape backs out of a picture, article or search. Page Up and Page Down scroll.
- On Mac, many app commands use Command in place of Ctrl. Settings > Controls shows and edits the bindings for your profile and editor context.
Related: Customize mouse and keyboard behavior | Restore a shortcut that stopped working

### Diagnose missing sound, files and plugins
Keywords: troubleshoot no sound blank audio disabled plugin missing file scan errors underruns reset
1. Verify PAT or SONG, an active pattern or arrangement, and an enabled audio output. Check Master and each source's routing and mute state.
2. Test a sample or instrument already known to play. If only one channel fails, look for a missing sample, missing plugin or a disabled effect that changes the route.
3. Check Audio settings and the underrun counter. Test a stable buffer and use the performance monitor to isolate a heavy plugin. Restart a disconnected interface before scanning unrelated plugins.
4. For a missing file, locate the original and relink it. Save a version after confirming it plays. For a plugin scan error, confirm format and architecture, add its installation folder and rescan errors.
5. If the issue persists, include Help > About's build, OS, driver, sample rate, buffer and steps in the bug report. Attach a small reproducible project if you can share its content.
Related: Fix delay, crackles or no sound | Report a problem and check for updates

### Project compatibility and feature availability
Keywords: FAQ FL Studio FLP lmn compatibility VST ARA native formats performance mode midi scripts versions platforms
Open supported FL Studio projects through File > Open. Compare routing, automation, sample references and third-party formats against the original. Import reports help locate missing items or behaviors that require attention. Keep the original file and save your ML Studio work as a separate .lmn.
Greyed menu entries are unavailable in the current build or context. Playlist Performance Mode and MIDI scripts are not implemented. Choose the supported MIDI Learn or controller profiles for hardware control. Availability of native drivers, ARA editors and optional tools depends on platform and installed components.
Use Help > About to compare builds and Help > Change Log to check what shipped in each release. Download and install the correct architecture for your OS.
Related: Import a project from FL Studio | Set up a keyboard or mixer controller | Install and open ML Studio


### Editing names and browsing files
Keywords: save open copy paste space clipboard filename path text selection browse samples folder

Open/Save, New project and Rename project text fields accept spaces and Unicode names. Use Ctrl+A/C/X/V (Command on macOS) to select all, copy, cut and paste; Ctrl+Z and Ctrl+Y or Ctrl+Shift+Z undo/redo text edits. Shift+Left/Right or Shift+Home/End selects part of the text, Ctrl+Left/Right moves by word, and Ctrl+Backspace/Delete removes a word. Click to position the caret or drag to select a range. Pasting into these single-line fields turns line breaks and tabs into spaces. These edits stay inside the dialog; they do not operate playback or project undo.

In a file browser, Ctrl+L focuses and selects the folder path; paste or type a path and press Enter to navigate. Alt+Up goes to the parent folder, Alt+Left goes back, F5 refreshes, and Ctrl+Shift+N creates a folder. Tab switches between name and path; Shift+Tab also moves backwards through the Save form’s expanded Information fields. Cancel or Esc closes without saving, and existing files still require overwrite confirmation. In the project Save form, the folder button chooses the location; Browse opens the regular file browser.

Right-click empty space in the Browser to load a sample into a new channel, add a sample folder, open the latest added sample folder in your system file manager (the user folder if no folder has been added), configure folders, or refresh. Loading a sample uses the usual new-sampler workflow and its project undo. Choosing a sample folder keeps it in the Browser for future use. Settings > Visibility > Browser > Empty-space file actions hides this shortcut menu without removing folders or project content.

### Chat with the optional ML Studio Helper (Melli)
Keywords: helper Melli companion AI chat offline memory model GPU assistant animation download install remove
Choose Help > ML Studio Helper (Melli). On first use, Download and install fetches the 492 MB Qwen3.5-0.8B UD-Q3_K_XL model directly from Hugging Face, plus about 175-245 MB of runtime files depending on your platform; allow 2 GB free disk space. Python and the local runtime are included. Supported targets are Windows x64, Linux x64 with glibc 2.36+ and a desktop environment, and macOS 14+ on Apple Silicon or Intel. The download is verified against hashes embedded in the DAW. A progress panel shows the current file, actual downloaded MB and percentage, with Cancel download and Run in background buttons. Open the same Help entry to see progress again. Verification and installation are shown as separate stages; an interrupted download can be retried. Nothing is downloaded just by opening ML Studio.

Click the character to load the model and open her speech bubble. Type in Talk to Melli and press Enter or the arrow. Melli is a witty, warm companion for everyday conversation as well as music, theory and ML Studio questions. Exact application instructions come from this build's bundled guide; ... > Guide source shows the consulted passages. She cannot browse the web, hear your project or operate your computer. Replies can still be wrong or select an irrelevant passage.

Minus collapses the bubble while keeping the model loaded. X closes Melli and unloads the model, including during a response. Closing ML Studio also stops her. Settings > Visibility > Workspace > ML Studio Helper companion hides the entry and stops the companion without changing project data. The overlay follows the main window and hides when minimized or another app is active; Wayland desktops may control positioning. GPU acceleration uses Vulkan on Windows/Linux or Metal on Mac, with a CPU fallback. Close Melli when you need her resources for demanding audio work.

Recent context is bounded to six exchanges / 12000 characters and is never saved as a transcript. ... > New chat or closing Melli clears it. Useful self-disclosed facts can persist separately as up to 24 short notes in one file no larger than 16 KiB. Older notes are displaced when full. ... > Memory lets you view, edit, disable learning/use, or Forget everything. Turning memory off preserves existing notes but stops using them. Memory is imperfect; do not share passwords or other secrets. Small error logs rotate, model files do not grow as you chat, and changing DAW versions reuses the model.

To reclaim space, choose ... > Remove downloaded Helper files and confirm in ML Studio. This removes package-owned runtime/model files and keeps saved notes; forget those first if desired. Reopen the Help entry to download Melli again. Ordinary installations use the ML Studio user folder's Melli subfolder. No account is required; after installation, chats work offline. Linux desktops must provide the Qt/X11 or Wayland system libraries. If Melli fails to open on Debian/Ubuntu, ensure libxcb-cursor0, libxkbcommon-x11-0, libegl1 and libopengl0 are installed through your system package manager. The portable files include Qt itself, not the operating system display server or graphics drivers.

### Select multiple Mixer channels
Keywords: select multiple mixer channels tracks inserts multiselect ctrl click cmd selection several group faders
Click an empty part of the first Mixer strip body, then hold Ctrl and click the body of each additional strip. On macOS use Command in place of Ctrl. This adds individual Mixer channels to the selection; they do not have to be next to each other. Use the strip body rather than the mute light, polarity button, meter or routing arrow, which perform their own actions.
Clicking an already-selected strip without Ctrl changes the current strip while keeping the group selected. Clicking an unselected strip without Ctrl replaces the group with that strip. Ctrl-click adds; it does not toggle a selected strip off. Shift-click range selection is not implemented by this Mixer strip gesture. The attached inspector's channel selector chooses a single channel.
Drag a selected channel's volume fader to move the selected unlocked channels together while preserving their relative gain relationships where the fader limits allow. A locked channel does not follow. Undo restores the group volume move. Selecting Mixer strips is separate from selecting Channel Rack instruments or Playlist rows.
Related: Set levels, pan and routing | Select and edit several channels | Lock a Mixer channel

### Route multiple Mixer channels to a bus
Keywords: mixer multiple channels group route routing bus send ctrl shift only sidechain remove master
Select the source Mixer strips with Ctrl-click. Click the destination's routing arrow to add a normal route from the selected sources. The destination is the channel that receives the audio. Master cannot be used as a source, and a channel cannot route to itself. Routing that would create a feedback loop is refused.
Ctrl-click the destination routing arrow for routing only to that destination, removing those sources' other routes. Shift-click the destination arrow for a sidechain send with a zero audible send level. Ctrl+Shift-click combines sidechain with only-to-this-destination routing. These modifiers apply to routing arrows, not to selecting the strip body.
For a drum bus, select the drum source inserts and use routing only to the bus when you want the bus to be their sole output path. Keeping both a direct Master route and a bus route creates parallel signal paths. For a reverb send, normally keep the original output and add a send to the reverb insert, then adjust the send amount. Locked sources/destinations can prevent changes; check the status message.
A sidechain route alone does not configure compression. Choose the external input in a plugin that supports it, or use Quick Sidechain. Save the project to retain routing. Use Undo for a mistaken routing edit.
Related: Select multiple Mixer channels | Create sends, buses and sidechains | Duck a track with Quick Sidechain

### Solo a Mixer selection and preserve its buses
Keywords: mixer solo selected channels group downstream upstream alt restore mute
Select the Mixer channels you want to hear, then use Solo selected tracks from the track context menu. Solo includes downstream destinations needed to hear the selected channels, including buses and Master. Alt with the Mixer solo gesture also includes upstream channels feeding the selected tracks.
The Restore previous state after solo preference determines whether leaving the solo restores the earlier enabled/muted state. If a required change would modify a locked channel, the operation is blocked and shows a lock message. This is different from deleting routes or bypassing effects; the routing itself stays in the project.
Related: Select multiple Mixer channels | Lock a Mixer channel | Create sends, buses and sidechains

### Insert and reorder Mixer tracks without losing routing
Keywords: mixer insert multiple tracks reorder move selected channels recording lock separator
Right-click an existing Mixer insert and choose Insert Multiple Tracks. Enter a valid count in the dialog and confirm. The available count depends on remaining Mixer capacity. Stop recording first; inserting tracks while recording or across a protected group is refused.
Moving selected Mixer inserts treats the selected tracks as a stable block. The tracks keep their routing and plugin identity as their positions change. Master is excluded from insert reordering. Locked channels/groups and recording can prevent a move. Use Undo to return from an unintended structural change.
Related: Rename, color and group mixer channels | Select multiple Mixer channels

### Choose normal sends, sidechains or Quick Sidechain
Keywords: routing sidechain duck kick bass compressor limiter key input send level threshold release all FX
A normal send carries audible signal to another Mixer insert. A sidechain supplies a detector/key signal to a compatible effect; it is not automatically an audible parallel mix. Select the trigger/source channel before operating the destination routing arrow. For manual setup, create the sidechain and select that external input in the destination plugin.
For Quick Sidechain, select the trigger, right-click the destination's routing arrow and choose Quick Sidechain. It adds ML Limiter in COMP mode, selects the external input, starts at -30 dB threshold, 4:1 ratio and 200 ms release, and enables destination effect processing. The trigger retains its other outputs. Open the limiter to adjust the amount and timing.
Quick Sidechain preserves existing effect order and uses slot 10 or a slot after the last occupied user slot. A full chain or routing loop is refused before changing the project. If All FX is bypassed, turn that audition bypass off to hear the ducking. The setup can be undone.
Related: Duck a track with Quick Sidechain | ML Limiter | Route multiple Mixer channels to a bus

### Expand and navigate a long Mixer effect chain
Keywords: mixer effects slots 63 add slot scroll move alt wheel EQ bypass wet dry
Filling the final visible user-effect slot reveals another empty slot. Use + Add slot to reveal one explicitly. A Mixer channel supports 63 user effects, separate from the dedicated mini EQ. The effects panel displays ten rows at a time and indicates the visible range.
Wheel over the effect list to reach additional slots. Alt+wheel or an effect's Move up/down menu changes its processing order. The small slot mix knob keeps its own wheel behavior. Slot moves skip the reserved EQ; the mini EQ pre/post setting remains relative to the entire user chain.
Save the project to retain expanded slots, plugin states, bypass and mix. Older builds limited to ten slots cannot play the additional effects correctly; use a compatible newer build when transferring such a project.
Related: Add, reorder and bypass effects | Use the Mixer mini EQ before or after effects

### Understand Channel Strip view processing and removal
Keywords: channel strip mixer rack original plugin native EQ strip bypass remove free slot fill all independent
Channel Strip view shows the original supported plugin interface above Lemon's fader, pan, mute/solo and meters. Choose an installed/scanned supported strip from a blank upper panel. The original vendor controls and preset menus remain available. General effect menus can still load other effects.
The assigned rack processes before that channel's remaining effects and replaces its native strip and dedicated EQ processing while assigned. The native settings are retained. The rack runs independently of the ordinary effects-enable switch; its own slot bypass still works. A free user-effect slot is required, and other effects are not overwritten.
Remove the strip assignment to restore native strip/EQ processing. The former rack stays as a bypassed effect. Changing racks also keeps the former rack bypassed. Save the project for assignments and plugin states; assignment/removal and batch changes support Undo.
Fill all insert channels with creates independent plugin instances. Copy this rack and settings to all inserts copies settings as well. These commands apply to insert channels; Master monitoring remains separate. If filling appears to do nothing, confirm an eligible scanned plugin and free slots, check the displayed loading error, and verify the selected view/context. Do not assume a silent failure is successful loading.
Related: Fill Mixer inserts with a channel strip | Choose a channel strip and use its processors | Use SSL 360 Link in Channel Strip view

### Page and resize original Channel Strip plugin editors
Keywords: channel strip paging Waves Scheps SSL EV2 E G TG12345 Magma RChannel REDD size DPI scaling scroll
Use the left/right buttons below a Channel Strip editor to page through complete sections. Supported layouts have section-aware stops; EV2 and TG12345 pages overlap so controls crossing a decorative divider remain available on at least one page. Unknown or expanded layouts fall back to the whole interface.
The plugin scales to available height when it supports host scaling. If it refuses scaling, its actual original dimensions are retained. Resize the Mixer to make room; if the editor is taller than the available screen, vertical scrolling is the fallback. Hiding or scrolling an editor out of view does not stop its audio processing.
Only installed/scanned supported entries appear in the strip picker. Entries in Other resizable strips (unsupported) are experimental. A vendor's own scaling/preset menus remain vendor controls; the guide cannot define every third-party interface version.
Related: Choose a channel strip and use its processors | Plugin manager and plugin editors

### Route shared audio clips independently
Keywords: audio clip shared sample channel routing mixer track picker make unique independent
Click and release the sample/channel TRACK box to open the miniature Mixer routing picker. The Channel Rack Mixer box and Playlist audio-clip menu Send to mixer track also open it. Wheel over the TRACK control changes its number. Dragging the control is a routing drag rather than a picker click, even if you return to the start.
Search by destination name or number. Click a tile to apply and close. Previous/Next and the wheel reach further tracks; Master 0 is included. Escape or the close button cancels without editing. The current destination is highlighted.
Routing belongs to the sample channel. Other Playlist clips sharing that channel follow the same change. Make the clip unique before changing routing when only that clip should move to another Mixer destination. Routing changes are undoable and saved with the project.
Related: Route audio with the miniature Mixer picker | Make a clip independent

### Decide between freeze, wet bounce and keeping the Mixer live
Keywords: freeze bounce consolidate CPU instrument mixer effects buses sidechain tails double processing
Freeze track (save CPU) on a Playlist header renders hosted instruments and unmuted clips to audio stems, retaining original clips for Unfreeze track (resume editing). Mixer inserts, buses and sidechains remain live. It saves instrument CPU, not Mixer-effect CPU. Instruments shared by another row may still need to run.
Bounce audio in new track renders insert and outgoing bus processing, creates the audio row below the source/take group, and mutes the source after success. Bounce and replace track uses the same row, preserves automation/take lanes and existing mute state, and can be undone. The live Master processes the bounced result.
Wet bounce with an external sidechain is refused when it cannot correctly include the required other-channel signal. Use freeze or Consolidate (keep mixer live) when the routing/sidechain must continue running. Do not add the original wet processing again to an already processed bounce.
Rendered files use unique names in the project's Rendered folder and include tails. A failed/cancelled render does not replace the source. Unfreeze restores originals at the row's current location and discards edits to the temporary freeze clips; unfreeze before changing comp takes or recording into that row.
Related: Freeze or bounce a track | Commit insert and bus processing to audio | Keep a session editable while reducing CPU

### Know what headphone correction changes
Keywords: headphones phones AutoEq monitor export bounce recording master bypass offline cache
Click the Master headphone icon, search for the exact headphone model/variant, and select it to download and activate its correction. The PHONES rocker toggles correction. Choose the correct impedance, pads and ANC variant where listed. A failed download leaves the previous correction in place; the chosen profile is cached for offline use.
Correction affects ML Studio's physical output, including playback, live notes and previews. It is not printed into exports, freeze/bounce files or Mixer recordings. It is separate from master effects, All FX, STRIP bypass and the miniature EQ. Bypass it for speakers or when another correction system is already active.
The model and on/off choice are preferences for the active profile, not per-project mix processing. A catalog match is a measured-model correction, not an individual calibration of your particular headphones or ears.
Related: Correct headphone monitoring | Export a finished song, stems or MIDI

### Diagnose Smart disable after an idle gap
Keywords: smart disable sleeping plugin tail silence wake automation editor CPU VST CLAP export
Enable Smart disable in Settings > Audio > Advanced > CPU. It is off by default. Compatible VST2/VST3 instances may sleep after four seconds of exact silence plus their reported finite tail and latency. Effects require silent main and sidechain inputs. Quiet decays and noise are not treated as silence.
Notes, MIDI, automation, parameter/preset edits, editor opening and transport changes wake an instance. Unknown/infinite tails, held notes/pedals, auxiliary-output instruments, ARA playback and open/busy editors can keep it awake. CLAP uses its own plugin-requested sleep/wake behavior.
Exports process fully regardless of this live preference. If an internal sequencer or autonomous modulation fails after a pause, turn Smart disable off and compare; the host cannot predict unreported future output. Hiding the setting in Visibility does not change its saved value.
Related: Save CPU with Smart disable | Fix delay, crackles or no sound

### Resolve Controls inheritance and gesture conflicts
Keywords: controls global context inheritance keyboard capture double triple click hover 350 500 drag escape
Settings > Controls stores bindings per profile. Select Global, Piano Roll, Playlist, Mixer, Channel Rack, Browser, Piano Roll: empty grid or Piano Roll: notes; then choose the event and modifiers. Capture records a keyboard key; Escape cancels capture. A new binding replaces the same context/event/modifier combination.
The specific Piano Roll note/grid context inherits Piano Roll, then Global. Restore event removes an override and returns to inheritance. Do nothing explicitly suppresses the event. Text fields/dialogs retain input, and Typing keyboard to piano takes precedence over plain note letters; disable it before using those letters for editing shortcuts.
Click and drag assignments are independent. A drag starts after five pixels of movement. Assigned left double/triple-click actions defer the single click for 350 ms; without them there is no multi-click wait. A custom Hover event fires once after 500 ms on a target. Activate window fires on later editor activation, not startup. These are assigned Controls events, separate from Open menus on hover.
Escape cancels a deferred click. After a native drag starts, Escape finishes it without closing the editor; use Undo if you need to revert the edit. Supported actions operate on the current selection, and unavailable commands report that they cannot run. Mixer volume actions respect locks and grouped faders.
Related: Customize mouse and keyboard behavior | Restore a shortcut that stopped working | Open menus by hovering

### Save, import and repair custom action sequences
Keywords: controls macro sequence append 32 undo saved named pin toolbar JSON import export references
Use Append in Settings > Controls to build up to 32 compatible direct editing actions. A sequence is one project Undo/Redo step, even with an Undo-history limit of one. Dialog, file, project-management and Undo/Redo commands must run alone. Mouse-down gestures also run alone and cannot be assigned to keyboard, wheel or hover events.
Give a binding a name and choose Save name to reuse it through Saved actions. Pin as toolbar button exposes up to four named Custom actions buttons, with full-name tooltips. Show the buttons through Workspace Visibility and the toolbar layout as needed; hiding them retains the bindings.
Saved opens bindings for editing. Circular or missing named references are rejected. If you restore a binding that another action references, repair the dependent action before using it. Export saves names, sequences and pinned state as JSON. Import validates the whole file before replacing bindings; Undo change restores earlier binding-settings edits, up to 32 in the current session.
Unknown action IDs imported from another version remain stored but report unavailable. Custom menu arrangements do not change the original menu action IDs used by these bindings.
Related: Build action sequences and toolbar buttons | Customize mouse and keyboard behavior

### Write a portable action script
Keywords: lsa script repeat name action ID import macro fifth velocity portable limits
A portable .lsa script is UTF-8 text. Select the target context/event in Settings > Controls first; capture the key if the event is Keyboard, then Import the script. Each nonempty line is an action ID, a name: directive, a comment starting with #, or repeat COUNT ACTION_ID. Find exact supported IDs in a Controls JSON export.
For example, use three lines: name: Fifth softer; repeat 7 piano.up; piano.velocity-down. Put each of those on a separate line in the file, without the semicolons. This raises selected notes seven semitones and reduces velocity one step. The expanded sequence can contain at most 32 direct editing actions.
These scripts run finite DAW editing actions. They cannot execute shell commands, access files, run arbitrary JavaScript/Lua, wait asynchronously or recursively call named actions. Controllers that emit keyboard events can use keyboard bindings; native gamepad/HID mapping is not included.
Related: Save, import and repair custom action sequences | Build action sequences and toolbar buttons

### Choose the correct Plugin Picker destination
Keywords: plugin picker F8 insert replace instrument effect destination Hybrid favorites search picture
Open the picker from an instrument Add/Replace command for instruments, or from an effect slot for effects. Right-click an empty effect slot targets that exact slot. F8 over the Channel Rack chooses instruments; F8 over the Mixer chooses effects. Elsewhere the default F8 opens Hybrid. A customized picker shortcut takes precedence.
The toolbar/general Plugin picker opens Hybrid for drag-and-drop. Click a plugin to add to the original destination, or drag it to the Rack/Playlist/Mixer slot. Search covers names, categories, vendors and formats. Stars save favorites; arrow keys select, Enter adds and Escape closes.
Right-click empty Rack space or its + area for Plugin picker - insert instrument. Right-click a channel name or its piano-roll preview for Plugin picker - replace instrument. Insert adds a separate channel; Replace changes the selected one. Verify the destination before replacing a sound.
Related: Search, favorite and manage the plugin picker | Add, replace and route an instrument

### Generate, replace and troubleshoot plugin preview pictures
Keywords: plugin picker preview thumbnail picture generate Windows cancel report missing blank GPU license
The picker uses built-in/catalog pictures when available. Hover for a larger preview. Choose picture assigns a PNG/JPEG to the selected plugin identity in the current profile, resized to at most 480 by 300. It does not upload the picture or open the plugin just for ordinary browsing.
On Windows, Generate previews processes installed plugins missing a picture using separate offscreen editor helpers. Existing pictures are preserved. First scan newly installed plugins through Manage plugins. Progress shows the current plugin; Cancel stops generation and keeps completed pictures. Run it again to process remaining or new plugins.
Generation requires stopped playback and cancels if playback starts. Each plugin gets a 25-second timeout. Preview report explains skipped entries. Some protected/GPU editors or activation dialogs cannot produce a usable image; no activation prompts are clicked automatically. Use Choose picture for these entries. Existing pictures work across platforms; automatic capture is Windows-only in this implementation.
Related: Search, favorite and manage the plugin picker | Find installed plugins

### Understand what collaboration shares and what stays local
Keywords: collaboration shared edits local device zoom layout plugins sample transfer follow host playback
A collaboration session shares project editing; each computer renders audio using its own installed plugins. Patterns, notes, clips, arrangements, channel settings, Mixer controls/routing, automation and exposed plugin parameter gestures are shared. Full plugin state/preset changes settle while playback is stopped; received instance/state changes can wait until the recipient stops.
Sample files transfer, but plugin binaries, licences and external plugin libraries do not. Both participants need compatible plugins and libraries. Matching project data does not guarantee matching audio if plugin versions or device settings differ.
Playback is independent unless a guest enables Follow host playback. Following tracks host start/stop and position with drift correction; it is not sample-accurate internet jamming, audio streaming or voice chat. Audio devices, window layout, zoom/scroll, active editors and Playlist lane folding/heights stay local.
Conflicting edits pause shared editing for the affected participant. Reopen Collaborate and choose Keep my version or Use shared version deliberately. The host must remain connected; End / leave closes the session, and ending hosting invalidates its invitation. Each participant keeps a local project.
Related: Collaborate on a project | Resolve collaboration conflicts and recover sessions

### Keep project formats, versions and recovery files straight
Keywords: lmn flp save as save new version templates backups zip compatibility renamed extension
New projects, Save As, Save New Version, new project-folder saves, templates and autosave backups use .lmn. Open accepts .lmn, legacy .flp and zipped projects. Saving an already-open .flp normally keeps that filename; Save As proposes .lmn and Save New Version creates a new .lmn sibling. Existing files are not batch-renamed.
Both extensions are recognized for recovery. A .lmn extension does not convert unsupported third-party plugins into built-ins or guarantee perfect import of every FL Studio feature. Keep the source project when importing and review missing plugins, routing, automation and playback.
Save to a new project folder or export a zipped project when transferring audio with the song. Plugins and external libraries may still need separate installation. Before deleting unused audio or overwriting a version, keep a recoverable project copy and verify the project's sample locations.
Related: Native projects, folders and templates | Import a project from FL Studio | Send a project with its audio files

### Use the guide reference without confusing defaults with saved settings
Keywords: reference defaults current profile settings unavailable menu disabled feature controls hints
The setting-reference articles list the build's declared defaults and choices. Your saved profile may use different values. Hiding a setting through Visibility does not switch the feature off; change its value in its normal settings page. Menu customization can also hide commands independently of feature visibility.
Control-hint references quote the editor's own hover instructions. Modifiers apply to the named control/context, not universally across the app. Menu-reference pages list discoverable commands, not a sequence to follow. Entries explicitly marked not implemented cannot be used. Context, selection, platform, installed plugins and locks may further restrict a working command.
The Helper cannot inspect your current selection, customized shortcut assignments or third-party plugin interface from a text question. Tell it the editor, control and what you are trying to do. If the guide lacks a verified workflow, it should say what information is missing instead of inventing a shortcut. Current source coverage records distinguish extracted labels from reviewed behavior.
Related: Browse or search the User Guide | Find a control that is missing or greyed out

### Follow or duck a parameter from an audio level
Keywords: follow audio level peak link duck source depth release envelope control
Open a supported parameter's context menu and choose Follow audio level. Choose Follow source to make it follow a Mixer insert's level, or Duck from source for inverted behavior. Choose the source insert from the submenu. This creates a project-local audio-level link without adding a controller-plugin slot.
Reopen the link menu to set Depth to 25%, 50%, 75% or 100%, and Release to 50, 150, 300 or 600 ms. A new link starts at full depth and 150 ms release. Remove audio-level link removes the connection and restores the saved base value. Undo restores a link change. This is a parameter-control link, distinct from an audible Mixer send or a compressor's external sidechain input.
Related: Map a MIDI knob or controller | Create sends, buses and sidechains

### Change existing Piano Roll note colors and note types
Keywords: piano roll color group selected notes slide glide portamento normal apply
Open the note-color selector to choose one of 16 color groups for drawing. Changing the drawing color alone does not recolor every existing note. Select notes first, then use Selected notes > Apply current color in that menu to recolor those notes.
The same Selected notes submenu changes existing notes to Normal notes, Slide / glide notes or Portamento notes. These entries require a note selection. The change is undoable. Audible glide/portamento also depends on the instrument's supported note behavior; a note type is not a guarantee that every third-party plugin implements it.
Related: Use Glide Notes and slide pitches | Draw, move and edit notes

### Select notes, a pitch range or a time range in Piano Roll
Keywords: piano roll select notes E Ctrl add keyboard pitch range ruler right drag stretch deselect
Choose the Select tool (E by default) and drag a box around notes; Ctrl adds to the selection. On the keyboard, Ctrl/Command+Shift-drag selects notes across the chosen pitches. These gestures select notes in the current Piano Roll context, not Mixer channels.
Drag the Piano Roll ruler to move the play start. Right-drag the ruler to select a time range. With Pencil or Brush, right-click empty grid to deselect. Alt+wheel over a note edits velocity; over empty grid it changes row height. If a custom Controls binding overrides a gesture, restore that event or use the assigned action.
With Stretch active, drag note ends to scale selected timing and lengths; dragging note bodies still moves notes. Right-click empty grid to turn Stretch off. Undo reverses an unintended completed edit.
Related: Use the Piano Roll editing tools | Customize mouse and keyboard behavior

### Draw, route and save ML Wave modulation
Keywords: ML Wave modulation LFO drawn free run one shot tempo sync macro matrix depth route
Open MODULATION beside SYNTH. Draw in any of the four LFO graphs; each curve has 65 interpolated points. Sine, Triangle, Ramp and Square replace the shape, and Undo restores the previous edit. LFO1 switches to Drawn automatically; choose Classic for the original shape selector.
Free sync uses Hz. Tempo divisions include straight, dotted and triplet values. Note loop restarts per note, Free run uses a continuously advancing shared clock, and One shot runs once then holds its last value. Tempo-relative timing does not imply locking the phase to song position.
Drag an LFO, envelope or macro label to an oscillator, filter or output destination knob. Hover SYNTH during the drag to reveal its knobs. New routes start at +25%; change signed depth in the matrix. Clear removes a route. Unsupported drops cancel and a full matrix reports that no slot is available.
There are eight routes, thirteen sources and twenty-one destinations. The first two rows retain restricted selectors for older automation compatibility; the six newer rows offer all choices. Four macros are host-automatable. Displays follow the newest active voice. Custom curves are saved in native plugin state; parameter-only presets cannot carry custom assets. Completing a curve edit can restart held notes/tails, so draw between notes.
Related: Design sounds with ML Wave | Build a patch in ML Wave

### Build and apply an ML Wave wavetable draft
Keywords: ML Wave table edit workshop harmonics frames tween normalize smooth invert reverse apply draft undo
Open TABLE EDIT at the top of Wave or EDIT TABLE in oscillator A/B. Draw in DRAW WAVEFORM and drag harmonic bars 1-64; harmonic phase and higher existing harmonics are preserved. Edits remain a draft until Apply to Oscillator.
Choose a frame in the strip or with Frame arrows. Add, duplicate, delete, move left/right, or insert a Tween between the selected frame and the next. Tables contain 1-128 frames. Normalize, Smooth, Invert and Reverse affect the selected frame. Workshop Undo/Redo retains up to 24 editing operations.
The stacked view highlights the selected frame and interpolated preview. Move its slider to inspect transitions, then Apply to Oscillator and use Position on SYNTH to hear the scan. Applying embeds the table in the native patch/project and changes only the chosen oscillator to wavetable mode. Apply between notes because restoring state can restart held notes/tails.
Unapplied drafts are not saved with the project and are lost when the instance unloads or is recreated. Reload discards the draft and reloads the oscillator. Apply or Reload a dirty draft before switching A/B in the workshop.
Related: Design sounds with ML Wave | Build a patch in ML Wave

### Extract a Wave table from a recording and save a user bank
Keywords: ML Wave extract recording root pitch zero crossing user bank table name save reload
Select a recorded sound on Wave's SYNTH page, then open EDIT TABLE or Reload in the workshop. Drag across RECORDING to select a section, set the output frame count, and choose Extract. Extraction uses the recording's Root pitch and nearby rising zero crossings. Complex/detuned audio or an inaccurate Root can need adjustment; this is not automatic multisample mapping or continuous pitch tracking.
Click USER BANK and TABLE NAME to type names. Ctrl+A replaces the field, Backspace removes characters, Enter finishes and Escape restores its previous text. Save to User Bank adds the table to SYNTH > Wavetables as Bank / Table. Existing files receive numbered alternatives instead of being overwritten.
Apply embeds the table in the native project/preset; saved bank files remain local user-library files. Keep the bank files if you want those choices available in other projects. The source recording remains in the instrument library.
Related: Build and apply an ML Wave wavetable draft | Build a patch in ML Wave

### Choose ML Wave recordings, root pitch and unison Blend
Keywords: ML Wave sounds basic waves recordings root pitch sample one shot unison blend position import
Use each oscillator's source dropdown for Basic Waves, Wavetables, available recorded libraries or Reference Saw. The next dropdown chooses the wave/recording; recorded instruments also offer patch selection. Longer inline lists have Previous/Next. Click a choice or outside to close. Only importing a new external file opens a file picker.
Recorded samples play once at keyboard-controlled pitch; they are not automatically looped or mapped across multisample zones. Root buttons change the source pitch by -12, -1, +1 or +12 semitones. Raising Root makes the same played key sound lower. Position, Bend and phase are wavetable controls; recorded playback still uses pitch, unison, detune, envelopes, filter and effects.
UNISON BLEND shows center versus outer voice amplitudes. With one or two voices there are no distinct center/outer groups to rebalance. Loaded audio is embedded in native saved sounds. Sample or Root changes can interrupt held notes; edit between notes. Included recording libraries depend on the installed edition; do not assume a source-only or public build includes personal libraries.
Related: Design sounds with ML Wave | Extract a Wave table from a recording and save a user bank

### Navigate ML Plasma panels, patch view and advanced controls
Keywords: ML Plasma Synth Effects Links Play Setup Panels Patch Simple Full More advanced modules
Plasma has Synth, Effects, Links, Play and Setup tabs. Synth and Effects can display Panels or Patch. Simple folds advanced controls behind More; Full shows them in place. A block's menu or double-click opens its advanced panel. Back returns to the main view.
Synth arranges Layer 1-3, Root, Texture, Filter 1/2 and Amp, with modulation sources below. A block's on light controls its contribution. Shift-click its name toggles it. Drag a block name to reorganize the screen; visual row order does not rewire sound. Copy/paste block settings only between compatible block kinds.
The Parts sidebar lists available Modules and library items. Add a free module by clicking or dragging it; a full family is unavailable. Removing a block switches its slot off and resets its settings. Library can load tables into layers and shapes into LFOs. Patch shows audio routing as wires and exposes modulation relationships; use Routing to edit the signal destinations deliberately.
Related: Design sounds with ML Plasma

### Choose ML Plasma layer engines and filter routing
Keywords: ML Plasma wavetable sample multisample granular spectral root texture filter main direct routing
Each of Plasma's three main layers can use wavetable, multisample, sample, granular or spectral synthesis. Root supplies the sub oscillator and Texture supplies noise. Choose the layer mode before adjusting its mode-specific controls; wavetable frame position is not the same as sample position or granular offset.
Granular controls include grain length, density, offset, direction, window shape and random variation in pitch, time, pan and gain. Spectral resynthesizes a sample or table. Root has shape, octave, coarse pitch, phase, retrigger, pan and level; Texture supports built-in noise or a file with one-shot/keytrack, color, start/random-start, pitch, fine, pan and level.
Choose Filter 1/2 type, cutoff, resonance, drive, key tracking, pan, mix and level. A type-dependent extra control changes with the model. Each source chooses F1, F2, F1+F2 with balance, MAIN or DIRECT. The Routing panel also sets source/filter outputs, send levels, pan and level. Watch gain when combining direct and filtered paths.
Related: Design sounds with ML Plasma | Navigate ML Plasma panels, patch view and advanced controls

### Edit ML Plasma effects, links and performance quality
Keywords: ML Plasma effects Main Send A Send B links 64 modulation matrix smoothing auxiliary quality oversampling
Effects has Main, Send A and Send B chains. Drag an effect by its name to reorder it or move it between chains. A chain's menu can copy, paste or empty it; Keep on preset load retains that chain when another sound loads. Open More or Full for the selected effect's detailed controls.
Links holds up to 64 modulation connections. Each has source, destination, amount, polarity, curve, rise/fall smoothing, live output and bypass. An auxiliary source has its own curve/invert and output scale. Select a link to edit its curve at full size; bypass or remove the intended link when a control moves unexpectedly.
Setup includes tuning, MPE, voice behavior and quality. Quality uses Draft without oversampling, High at 2x (default) or Ultra at 4x. More active layers, voices/unison, effects and oversampling increase processing cost. Turn unused parts off and compare quality settings in the musical context.
Related: Design sounds with ML Plasma | Find out why a knob moves when playback starts

### Program ML Plasma arpeggiator and phrases
Keywords: ML Plasma arp arpeggiator phrases latch thru gate chance trigger octave record layer extend fold
Open Play for the Arpeggiator and Phrases. The arpeggiator has twelve pattern slots, including up/down/as-played/chords/random and drawn steps with velocity/accent. Choose tempo division or Hz, dotted/triplet timing, transpose/range/cycles, gate, repeats, chance, latch/thru and retrigger behavior. Trigger-octave keys choose slots while playing.
Phrases provides twelve sequences with Piano Roll, velocity and parameter automation lanes. Set length, keyboard span, transpose, play mode, rate, launch quantization, restart and gate. Record captures played notes; Layer keeps existing notes, Extend lets the phrase grow, and the grid control quantizes. Fold shows used keys; the headphone control auditions placed notes.
These are Plasma's internal sequencers, separate from the DAW's Playlist and the Sampler wrench arpeggiator. Check which sequencer is enabled when repeated notes continue. MIDI-output availability also depends on host routing; do not assume enabling an internal output option connects an external device.
Related: Design sounds with ML Plasma | Set up Sampler arpeggios, echoes and tracking

### Import Plasma presets and investigate missing assets
Keywords: ML Plasma Serum SerumPreset fxp import presets library missing samples wavetables approximation
Use Plasma's Library or Open preset to load its native sounds, Serum 2 .SerumPreset or Serum 1 .fxp preset files. Imported settings are mapped to Plasma's own engine. The result is not a promise of identical Serum sound. Host-level missing-plugin replacement behavior must be checked separately; importing a preset does not establish that an entire foreign project is compatible.
Add your preset/content folders to the library when needed and use Read again to rescan. Referenced wavetables, samples and impulse responses must exist in your installed content folders. Read the import notice and hover the info mark beside the preset name for the detailed approximation/missing-file report.
Save the adapted sound under a new name after reviewing it. Plasma does not supply another vendor's proprietary content with an imported preset. If a source, warp or trigger mode has no exact counterpart, use the report to decide what to adjust rather than assuming an exact conversion.
Related: Design sounds with ML Plasma | Recover a missing plugin or preset

### Adjust Master Assistant controls precisely
Keywords: master assistant detailed controls numeric double click wheel threshold ratio attack release ceiling imager mono
In Detailed controls, Dynamic EQ exposes Threshold and Max reduction for four bands. Multiband Comp selects Low, Low mid, High mid or High with Threshold, Ratio, Attack, Release and Makeup, plus crossover frequencies. Stereo Imager has four width controls: 100% is unchanged and 0% is mono. Maximizer has Threshold, Ceiling, Release and mode.
Drag vertically or type a value. Double-click returns a control to its parameter default. Wheel editing is disabled on these knobs so scrolling the workspace does not accidentally change the sound. The controls remain saved and automatable processor parameters. Opening the editor alone does not enable processing; use its on/off control and compare bypass at sensible levels.
Related: ML Master Assistant detailed controls | Master a mix with Master Assistant

<!-- source-reference-start -->
### Setting: Main GUI scaling
Keywords: setting: main gui scaling reference controls options settings
Location: Settings > General > Display.

Available choices: System; 100%; 125%; 150%; 175%; 200%; 250%; 300%; 350%; 400%.
Default: System.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Pop-ups scaling
Keywords: setting: pop-ups scaling reference controls options settings
Location: Settings > General > Display.

Available choices: Main; 100%; 125%; 150%; 175%; 200%; 250%; 300%; 350%; 400%.
Default: Main.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Toolbars scaling
Keywords: setting: toolbars scaling reference controls options settings
Location: Settings > General > Display.

Available choices: Main; 100%; 125%; 150%; 175%; 200%; 250%; 300%; 350%; 400%.
Default: Main.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Colorful waveforms
Keywords: setting: colorful waveforms reference controls options settings
Location: Settings > General > Display.

Available choices: Off; Color map; Multiband.
Default: Off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Mixer style
Keywords: setting: mixer style reference controls options settings
Available choices: Original; Clean Modern; Analog.
Default: Analog.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Playlist scrollbar style
Keywords: setting: playlist scrollbar style reference controls options settings
Available choices: Classic; Separate thumb.
Default: Separate thumb.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Interface language
Keywords: setting: interface language reference controls options settings
Available choices: English; Français; Español; Deutsch; Português (Brasil); Italiano.
Default: English.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Note names
Keywords: setting: note names reference controls options settings
Location: Settings > General > Language.

Available choices: English; Germanic; Solfege (do, re, mi, ...).
Default: English.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Black notes
Keywords: setting: black notes reference controls options settings
Location: Settings > General > Language.

Available choices: Sharps - #; Flats - ♭.
Default: Sharps - #.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Autosave
Keywords: setting: autosave reference controls options settings
Location: Settings > File > Backup.

Available choices: Never (just remind every 10 minutes); Rarely (every 15 minutes); Occasionally (every 10 minutes); Regularly (every 5 minutes); Frequently (every 5 minutes & before risky operations); Very frequently (every minute & before risky operations).
Default: Occasionally (every 10 minutes).
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Location
Keywords: setting: location reference controls options settings
Location: Settings > File > Backup.

Available choices: Always save to the global folder; Save to the project data folder if there is one.
Default: Save to the project data folder if there is one.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Maximum
Keywords: setting: maximum reference controls options settings
Location: Settings > File > Backup.

Default: 100. Allowed control values: 1 to 999.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Restore previous state after solo
Keywords: setting: restore previous state after solo reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Select first channel with notes when the pattern changes
Keywords: setting: select first channel with notes when the pattern changes reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto zoom in piano roll
Keywords: setting: auto zoom in piano roll reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Brighter Playlist clips
Keywords: setting: brighter playlist clips reference controls options settings
Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Small scrollbars in editors
Keywords: setting: small scrollbars in editors reference controls options settings
Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto zip empty channels
Keywords: setting: auto zip empty channels reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Monophonic painting in piano roll
Keywords: setting: monophonic painting in piano roll reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Enable automatic crossfades in new projects
Keywords: setting: enable automatic crossfades in new projects reference controls options settings
Location: Settings > Project > Playlist.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Play truncated notes on transport
Keywords: setting: play truncated notes on transport reference controls options settings
Location: Settings > Audio > Mixer.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Reset plugins on transport
Keywords: setting: reset plugins on transport reference controls options settings
Location: Settings > Audio > Mixer.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Alternate meter scale
Keywords: setting: alternate meter scale reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Create automation clips at the playback position
Keywords: setting: create automation clips at the playback position reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto select linked modules
Keywords: setting: auto select linked modules reference controls options settings
Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Hide plugin window toolbar by default
Keywords: setting: hide plugin window toolbar by default reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto-route Channel Rack samples to mixer inserts
Keywords: setting: auto-route channel rack samples to mixer inserts reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto name channels
Keywords: setting: auto name channels reference controls options settings
Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto name effect slots
Keywords: setting: auto name effect slots reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Detach all new plugins
Keywords: setting: detach all new plugins reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Open detached plugins with the main window
Keywords: setting: open detached plugins with the main window reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Disable typing keyboard priority by default
Keywords: setting: disable typing keyboard priority by default reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Limit the number of open plugin editors
Keywords: setting: limit the number of open plugin editors reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Show startup splash screen
Keywords: setting: show startup splash screen reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Silent startup
Keywords: setting: silent startup reference controls options settings
Location: Settings > General > Miscellaneous.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Use both mouse buttons in step sequencer
Keywords: setting: use both mouse buttons in step sequencer reference controls options settings
Location: Settings > General > Input.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Startup project
Keywords: setting: startup project reference controls options settings
Location: Settings > General > Miscellaneous.

Available choices: Empty project; Default template; Last used project.
Default: Default template.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Click-and-hold & special gesture functions
Keywords: setting: click-and-hold & special gesture functions reference controls options settings
Location: Settings > General > Input.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Maximum undo levels
Keywords: setting: maximum undo levels reference controls options settings
Location: Settings > General > Undo history.

Default: 100. Allowed control values: 1 to 999.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Undo knob tweaks
Keywords: setting: undo knob tweaks reference controls options settings
Location: Settings > General > Undo history.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Alternate undo mode
Keywords: setting: alternate undo mode reference controls options settings
Location: Settings > General > Undo history.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Put undone recordings in the recycle bin
Keywords: setting: put undone recordings in the recycle bin reference controls options settings
Location: Settings > General > Undo history.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Show undo history in the browser when undoing changes
Keywords: setting: show undo history in the browser when undoing changes reference controls options settings
Location: Settings > General > Undo history.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Show recent changes at the top
Keywords: setting: show recent changes at the top reference controls options settings
Location: Settings > General > Undo history.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Read sample root note
Keywords: setting: read sample root note reference controls options settings
Location: Settings > General > Advanced.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Read sample tempo information
Keywords: setting: read sample tempo information reference controls options settings
Location: Settings > General > Advanced.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Animation refresh rate
Keywords: setting: animation refresh rate reference controls options settings
Location: Settings > General > Display.

Available choices: Less smooth; Smooth; Ultrasmooth.
Default: Ultrasmooth.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Don't limit windows to screen
Keywords: setting: don't limit windows to screen reference controls options settings
Location: Settings > General > Advanced.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Initialize controls automatically
Keywords: setting: initialize controls automatically reference controls options settings
Location: Settings > General > Advanced.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Fast sample preview
Keywords: setting: fast sample preview reference controls options settings
Location: Settings > General > Advanced.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto keep long audio on disk
Keywords: setting: auto keep long audio on disk reference controls options settings
Location: Settings > General > Advanced.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Threaded project sample loading
Keywords: setting: threaded project sample loading reference controls options settings
Location: Settings > General > Advanced.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Cursors scaling
Keywords: setting: cursors scaling reference controls options settings
Location: Settings > General > Advanced.

Available choices: System; 100%; 125%; 150%; 175%; 200%; 250%; 300%; 350%; 400%.
Default: System.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Legacy scaling
Keywords: setting: legacy scaling reference controls options settings
Location: Settings > General > Advanced.

Available choices: Main; 100%; 200%; 300%; 400%.
Default: Main.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: High visibility
Keywords: setting: high visibility reference controls options settings
Location: Settings > General > Display.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Force refreshes
Keywords: setting: force refreshes reference controls options settings
Location: Settings > General > Display.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Thick lines
Keywords: setting: thick lines reference controls options settings
Location: Settings > General > Display.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Transparent menus
Keywords: setting: transparent menus reference controls options settings
Location: Settings > General > Display.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Transparent windows
Keywords: setting: transparent windows reference controls options settings
Location: Settings > General > Display.

Available choices: Off; On.
Default: Off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Optimize for
Keywords: setting: optimize for reference controls options settings
Location: Settings > General > Input.

Available choices: Multi-button mouse; Pencil; Multi-touch monitor.
Default: Multi-button mouse.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Invert pencil buttons
Keywords: setting: invert pencil buttons reference controls options settings
Location: Settings > General > Input.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto close
Keywords: setting: auto close reference controls options settings
Location: Settings > Audio > Input / output.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Smart disable
Keywords: setting: smart disable reference controls options settings
Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Align tick lengths
Keywords: setting: align tick lengths reference controls options settings
Location: Settings > Audio > CPU.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Multithreaded generator processing
Keywords: setting: multithreaded generator processing reference controls options settings
Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Multithreaded mixer processing
Keywords: setting: multithreaded mixer processing reference controls options settings
Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Plugin delay compensation during playback
Keywords: setting: plugin delay compensation during playback reference controls options settings
Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Audio worker limit
Keywords: setting: audio worker limit reference controls options settings
Available choices: Automatic; 1; 2; 4; 6; 8.
Default: Automatic.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Audio processing priority
Keywords: setting: audio processing priority reference controls options settings
Available choices: Normal; High; Realtime.
Default: Realtime.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Recording format
Keywords: setting: recording format reference controls options settings
Available choices: 32-bit float; 24-bit PCM; 16-bit PCM.
Default: 32-bit float.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Recording offset
Keywords: setting: recording offset reference controls options settings
Default: 2000. Allowed control values: 0 to 4000.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Playback resampling quality
Keywords: setting: playback resampling quality reference controls options settings
Available choices: 2-point linear; 16-point sinc; 24-point sinc; 32-point sinc; 64-point sinc; 128-point sinc; 256-point sinc; 512-point sinc.
Default: 16-point sinc.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Project data size warning
Keywords: setting: project data size warning reference controls options settings
Location: Settings > General > Hints and warnings.

Available choices: 50MB; 100MB (default); 250MB; 500MB; 1GB; 1.5GB; Don't warn me.
Default: 100MB (default).
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Ask for data folder
Keywords: setting: ask for data folder reference controls options settings
Location: Settings > Project > Data folder.

Available choices: On startup, when starting a new project and when saving unsaved projects; When starting a new project and when saving unsaved projects; When starting a new project only; When saving unsaved projects only; Don't ask.
Default: When saving unsaved projects only.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Copy used samples to the project folder
Keywords: setting: copy used samples to the project folder reference controls options settings
Location: Settings > Project > Data folder.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Pickup (takeover mode)
Keywords: setting: pickup (takeover mode) reference controls options settings
Location: Settings > MIDI > Input.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Auto accept detected controller
Keywords: setting: auto accept detected controller reference controls options settings
Location: Settings > MIDI > Input.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Foot pedal controls note off
Keywords: setting: foot pedal controls note off reference controls options settings
Location: Settings > MIDI > Input.

Checkbox; default: on.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Toggle on release
Keywords: setting: toggle on release reference controls options settings
Location: Settings > MIDI > Input.

Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Link note on velocity to
Keywords: setting: link note on velocity to reference controls options settings
Location: Settings > MIDI > Input.

Available choices: (none); Velocity; Mod X; Mod Y.
Default: Velocity.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Link release velocity to
Keywords: setting: link release velocity to reference controls options settings
Location: Settings > MIDI > Input.

Available choices: (none); Release.
Default: Release.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Controller type
Keywords: setting: controller type reference controls options settings
Location: Settings > MIDI > Input.

Available choices: (generic controller).
Default: (generic controller).
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Synchronization type
Keywords: setting: synchronization type reference controls options settings
Location: Settings > MIDI > Output.

Available choices: MIDI clock; MTC 24 fps; MTC 25 fps; MTC 30 fps (drop); MTC 30 fps (non-drop); MTC 29.97 fps (drop); MTC 29.97 fps (non-drop).
Default: MIDI clock.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Omni preview MIDI channel
Keywords: setting: omni preview midi channel reference controls options settings
Location: Settings > MIDI > Input.

Default: 0. Allowed control values: 0 to 16.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Song marker jump MIDI channel
Keywords: setting: song marker jump midi channel reference controls options settings
Default: 0. Allowed control values: 0 to 16.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Generator muting MIDI channel
Keywords: setting: generator muting midi channel reference controls options settings
Default: 0. Allowed control values: 0 to 16.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Double-click resets faders and knobs
Keywords: setting: double-click resets faders and knobs reference controls options settings
Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Open menus on hover
Keywords: setting: open menus on hover reference controls options settings
Checkbox; default: off.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Piano Roll zoom keys
Keywords: setting: piano roll zoom keys reference controls options settings
Available choices: Page Up / Page Down; Ctrl + Page keys; Alt + Page keys; Shift + Page keys.
Default: Page Up / Page Down.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Vertical scrolling speed
Keywords: setting: vertical scrolling speed reference controls options settings
Location: Settings > General > Input.

Default: 256. Allowed control values: 0 to 512. Wheel increment: 32.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Horizontal scrolling speed
Keywords: setting: horizontal scrolling speed reference controls options settings
Location: Settings > General > Input.

Default: 256. Allowed control values: 0 to 512. Wheel increment: 32.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Smoothing
Keywords: setting: smoothing reference controls options settings
Location: Settings > General > Display.

Default: 128. Allowed control values: 0 to 512. Wheel increment: 16.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Browser preview track
Keywords: setting: browser preview track reference controls options settings
Location: Settings > Audio > Mixer.

Default: 0. Allowed control values: 0 to 125.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Metronome track
Keywords: setting: metronome track reference controls options settings
Location: Settings > Audio > Mixer.

Default: 0. Allowed control values: 0 to 125.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Toggle browser
Keywords: setting: toggle browser reference controls options settings
Available choices: F8; Shift+F8; Ctrl+F8; Alt+F8.
Default: Alt+F8.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Toggle plugin picker
Keywords: setting: toggle plugin picker reference controls options settings
Available choices: F8; Shift+F8; Ctrl+F8; Alt+F8.
Default: F8.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Toggle project picker
Keywords: setting: toggle project picker reference controls options settings
Available choices: F8; Shift+F8; Ctrl+F8; Alt+F8.
Default: Ctrl+F8.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Show plugin database
Keywords: setting: show plugin database reference controls options settings
Available choices: F8; Shift+F8; Ctrl+F8; Alt+F8.
Default: Shift+F8.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Show when necessary
Keywords: setting: show when necessary reference controls options settings
Available choices: (none); Alt; Shift.
Default: (none).
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Show every time
Keywords: setting: show every time reference controls options settings
Available choices: (none); Alt; Shift.
Default: Alt.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Don't show
Keywords: setting: don't show reference controls options settings
Available choices: (none); Alt; Shift.
Default: Shift.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Show clip properties
Keywords: setting: show clip properties reference controls options settings
Available choices: (disabled); (none); Alt.
Default: Alt.
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### Setting: Show channel settings
Keywords: setting: show channel settings reference controls options settings
Available choices: (disabled); (none); Alt.
Default: (none).
This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.

### App - menu labels 1
Keywords: app - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Editor size [submenu]
- Replace [submenu]
- Refresh [submenu]
- View voice count
- View FPS
- Misc
- Performance monitor style [submenu]
- Detailed table
- Load dashboard
- Plugin performance monitor
- Audio settings
- Continuous scrolling
- Default device
- Type in BPM...
- Tap tempo

### Ara - menu labels 1
Keywords: ara - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Open current editor
- Remove ARA edits (use original audio)
- Scan for ARA editors...

### Browser - menu labels 1
Keywords: browser - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Scan for ARA editors...
- System [submenu]
- Tags [submenu]
- Add to channel rack
- Add to selected mixer track
- Load into selected channel
- Collapse structure
- Reread structure
- Content [submenu]
- Browser tabs [submenu]
- Audio preview [submenu]
- Appearance [submenu]
- Search [submenu]
- Navigation [submenu]
- Send to selected channel [submenu]

### Browser Preview - control hints 1
Keywords: browser preview - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Click waveform to preview from here

### Harmony - control hints 1
Keywords: harmony - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Chord workspace
- Select this chord
- Enter accepts; Escape cancels

### Changelog - control hints 1
Keywords: changelog - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Search the Change Log

### Wiki - control hints 1
Keywords: wiki - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- User Guide home
- Search the User Guide (Ctrl+K)
- Enlarge picture
- Close picture

### Import Report - menu labels 1
Keywords: import report - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Show import report
- Export full report...
- Find missing samples...
- Use available plugin substitutes (default presets)
- Browse for another file...

### Instrument Stack - menu labels 1
Keywords: instrument stack - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Remove instrument...
- Channel instruments [submenu]

### Knobs - menu labels 1
Keywords: knobs - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Reset

### Loop Fit - menu labels 1
Keywords: loop fit - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Fit to length [submenu]

### Messages - menu labels 1
Keywords: messages - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Show all
- Show none

### Midi Play - menu labels 1
Keywords: midi play - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Off

### Mini Eq - control hints 1
Keywords: mini eq - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Open the full Lemon Parametric EQ — same settings

### Misc - menu labels 1
Keywords: misc - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Reset

### Mixer - menu labels 1
Keywords: mixer - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Off
- Send to this channel
- Route to this channel only
- Sidechain to this channel
- Route to this track
- Route to this track only
- Quick Sidechain
- Sidechain to this track
- Sidechain to this track only
- Custom color...
- Insert Multiple Tracks
- Rename...
- Save effect chain…
- Change color... [submenu]
- Reset selected track(s) to default
- Reset selected track(s) routing

### Mixer - menu labels 2
Keywords: mixer - menu labels 2 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Separator
- Channel Strip mixer [submenu]
- Channel layout [submenu]
- Dock to [submenu]
- File [submenu]
- Move up
- Move down
- Inspector style [submenu]
- Show in inspector [submenu]
- Section order [submenu]
- Reset inspector layout
- Inspector on left
- Attached inspector layout [submenu]
- Plugin delay compensation [submenu]
- View [submenu]
- Detached

### Mixer - menu labels 3
Keywords: mixer - menu labels 3 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Trim unused tracks [submenu]
- Remove all (audio editor) instances
- Vintage compact mixer
- Full Original mixer — vintage
- Channel Strip mixer
- Replace [submenu]
- Delete...
- Presets [submenu]
- Browse parameters
- Sidechain input [submenu]
- View editor
- Rename, color and icon...
- Scaling [submenu]
- None
- Remove
- Cancel

### Mixer - unavailable menu placeholders 1
Keywords: mixer - unavailable menu placeholders 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Change icon... [not implemented]
- Left [not implemented]
- Middle [not implemented]
- Right [not implemented]
- Render to wave file... [not implemented]
- Allow threaded processing [not implemented]
- Automatic [not implemented]
- Link all parameters... [not implemented]
- Waveforms [not implemented]
- Extra properties [not implemented]
- Switch smart disable for all plugins [not implemented]
- Plugin database [not implemented]
- Patcherize [not implemented]
- Patcherize all [not implemented]
- Browse presets [not implemented]
- Save preset as... [not implemented]

### Mixer - unavailable menu placeholders 2
Keywords: mixer - unavailable menu placeholders 2 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Add to plugin database (flag as favorite) [not implemented]
- Display preset name in preset selector [not implemented]
- Store in spare state [not implemented]
- Flip with spare state [not implemented]
- Smart disable [not implemented]
- Random color [not implemented]
- 100% [not implemented]
- Help [not implemented]

### Mixer Recording - menu labels 1
Keywords: mixer recording - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- None
- No audio inputs found: check Settings > Audio

### Mpe - menu labels 1
Keywords: mpe - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- MPE / Note expression [submenu]
- Zone [submenu]
- Member channels [submenu]
- Pitch bend range [submenu]
- Pitch sensitivity [submenu]
- Pressure response [submenu]
- Slide response [submenu]

### One Click - menu labels 1
Keywords: one click - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Into the audio editor
- (no audio inputs)

### Params - menu labels 1
Keywords: params - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Edit automation clip...
- Create automation clip
- Copy value
- Paste value
- Edit events in new window
- Edit events in piano roll
- Type in value...
- Set... [submenu]

### Params - unavailable menu placeholders 1
Keywords: params - unavailable menu placeholders 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Edit events [not implemented]
- Override volatile link... [not implemented]

### Peak Link - menu labels 1
Keywords: peak link - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Follow audio level...
- Follow source [submenu]
- Duck from source [submenu]
- Depth [submenu]
- Release [submenu]
- Remove audio-level link

### Piano Note Mode - control hints 1
Keywords: piano note mode - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Note color: choose a color group; apply color/type to selected notes
- Scale Notes Only: hide out-of-scale keys

### Piano Note Mode - menu labels 1
Keywords: piano note mode - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Apply current color
- Normal notes
- Slide / glide notes
- Portamento notes
- Selected notes [submenu]

### Piano Tools - menu labels 1
Keywords: piano tools - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Import MIDI file...
- Export as MIDI file...
- Chord workspace...
- Key and scale...
- Send Selected Notes To... [submenu]
- Move to channel [submenu]
- Copy to channel [submenu]
- Key, scale and highlighting...
- File [submenu]
- Edit [submenu]
- Tools [submenu]
- Stamp [submenu]
- Select [submenu]
- Snap [submenu]
- Note length [submenu]
- Draw mode [submenu]

### Piano Tools - menu labels 2
Keywords: piano tools - menu labels 2 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- View [submenu]
- Keyboard style [submenu]
- Control lane [submenu]
- Helpers [submenu]
- Time markers [submenu]
- MIDI parts [submenu]
- Target channel [submenu]
- Keyboard width [submenu]

### Piano Tools - unavailable menu placeholders 1
Keywords: piano tools - unavailable menu placeholders 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Auto locate [not implemented]

### Playlist - menu labels 1
Keywords: playlist - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Edit [submenu]
- Undo
- Cut
- Copy
- Paste
- Duplicate
- Delete
- Shift left
- Shift right
- Shift up
- Shift down
- Make unique
- Mute
- Unmute
- Insert space
- Delete space

### Playlist - menu labels 2
Keywords: playlist - menu labels 2 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Tools [submenu]
- Consolidate playlist selection
- Export all playlist tracks
- Quick quantize start times
- Tool [submenu]
- Snap [submenu]
- Visible clip controls [submenu]
- Audio clip options [submenu]
- Automation editing [submenu]
- Pattern note colors [submenu]
- Select [submenu]
- Deselect
- Select all
- Select by selected source
- Select muted
- Invert selection

### Playlist - menu labels 3
Keywords: playlist - menu labels 3 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Select time around selection
- Group [submenu]
- Group
- Ungroup
- Zoom [submenu]
- Zoom in
- Zoom out
- Zoom out far
- Zoom on selection
- Time markers [submenu]
- Reset empty playlist tracks
- Clip options [submenu]
- View [submenu]
- Waveform detail when zoomed [submenu]
- Track cleanup [submenu]
- Marker profiles [submenu]

### Playlist - menu labels 4
Keywords: playlist - menu labels 4 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Save profile...
- Load and replace markers...
- Merge profile...
- Manage profiles...
- Add one...
- Add auto
- Add one every
- Add jump to next bar
- Add time signature change...
- Place loop
- Change to marker loop
- Use this track's instrument
- Detect sample tempo
- Make it fit
- Half length (double time)
- Double length (half time)

### Playlist - menu labels 5
Keywords: playlist - menu labels 5 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Fit to bars [submenu]
- Reset to original duration
- Stretch clip edges
- Trim clip edges
- Edit audio timing
- Preview
- Muted
- Rename...
- Make Unique
- Send to mixer track...
- Clip effects...
- Change color...
- Vocal Align to this clip…
- Time / stretching [submenu]
- Audio processing [submenu]
- Strip silence...

### Playlist - menu labels 6
Keywords: playlist - menu labels 6 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Normalize audio clips [submenu]
- Edit with ARA...
- Stem separation...
- Clean up audio in Tesla...
- Select & organize [submenu]
- Select all similar clips
- Random color
- Use selected time range in comp
- Comping [submenu]
- Use entire take in comp
- Edit selected MIDI clips together
- Edit pattern
- Reset pattern stretch
- Cut itself
- Copy value
- Return to original value

### Playlist - menu labels 7
Keywords: playlist - menu labels 7 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Paste value
- Type in value...
- Create folder
- Dissolve folder (keep tracks)
- Unfreeze track (resume editing)
- Freeze track (save CPU)
- Bounce audio [submenu]
- Bounce audio in new track
- Bounce and replace track
- Create take lanes / comp
- Color [submenu]
- This track height [submenu]
- Use default height
- Compact
- Normal
- Tall

### Playlist - menu labels 8
Keywords: playlist - menu labels 8 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Extra tall
- Playlist row height (all tracks) [submenu]
- Mute all clips
- Unmute all clips
- Group with above track
- Collapse group
- Insert one
- Move up
- Move down
- Track type and routing [submenu]
- Add marker...
- Add time signature...
- Move content left
- Move content right
- Reset
- Create automation clip

### Playlist - menu labels 9
Keywords: playlist - menu labels 9 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Clone...
- Merge with...
- Merge with... [submenu]
- Browse parameters
- Route to free mixer track
- Create automation [submenu]

### Playlist - unavailable menu placeholders 1
Keywords: playlist - unavailable menu placeholders 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Off [not implemented]
- Beats [not implemented]
- Bars [not implemented]
- 2 bars [not implemented]
- 4 bars [not implemented]
- 8 bars [not implemented]
- Quantize song jumps [not implemented]
- Add two... [not implemented]
- Root note [not implemented]
- Scale [not implemented]
- Add key marker [not implemented]
- Change icon... [not implemented]
- Edit sample... [not implemented]
- Edit events [not implemented]
- Piano roll [not implemented]
- Replace [not implemented]

### Playlist - unavailable menu placeholders 2
Keywords: playlist - unavailable menu placeholders 2 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Delete... [not implemented]
- Presets [not implemented]
- Browse presets [not implemented]
- Save preset as... [not implemented]
- Add to plugin database (flag as favorite) [not implemented]
- Store in spare state [not implemented]
- Flip with spare state [not implemented]
- Link all parameters... [not implemented]
- Allow threaded processing [not implemented]
- Smart disable [not implemented]
- Rename, color and icon... [not implemented]
- Save channel state as... [not implemented]
- Browse channel states [not implemented]

### Playlist Controls - menu labels 1
Keywords: playlist controls - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Stretch audio when resizing

### Playlist Links - menu labels 1
Keywords: playlist links - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Audio track output [submenu]
- Instrument track [submenu]
- Inspect current clip outputs [submenu]

### Playlist Tools - menu labels 1
Keywords: playlist tools - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- No clips yet
- Show clip picker panel

### Drops - menu labels 1
Keywords: drops - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Drop audio on track headers [submenu]
- Cancel

### Plugin Downloads - control hints 1
Keywords: plugin downloads - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Plugin Downloads

### Plugin Menu - menu labels 1
Keywords: plugin menu - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Manage plugins...
- Organize plugin folders [submenu]
- Effects
- Instruments
- Open plugin picker

### Plugin Picker - control hints 1
Keywords: plugin picker - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Restore plugin picker
- Plugin picker
- Drag to move Plugin Picker
- Drag to resize Plugin Picker

### Presets - menu labels 1
Keywords: presets - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- (no presets)
- Presets
- Browse presets
- Save preset as...
- Import preset...
- Rename preset...
- Delete preset...

### Project Info - control hints 1
Keywords: project info - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Click to close

### Project Picker - menu labels 1
Keywords: project picker - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- New pattern
- (no channels)

### Rack - menu labels 1
Keywords: rack - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Add instrument [submenu]
- Edit channels [submenu]
- Color selected [submenu]
- Organize [submenu]
- Display filter [submenu]
- Sort by [submenu]
- Mixer routing [submenu]
- Audio tools [submenu]
- View [submenu]
- Auto resize [submenu]
- Channel button width [submenu]
- Channel name alignment [submenu]
- Plugin picker - insert instrument...
- Insert instrument [submenu]
- Plugin picker - replace instrument...
- Tempo / stretching [submenu]

### Rack - menu labels 2
Keywords: rack - menu labels 2 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Replace instrument [submenu]
- Channel instruments [submenu]
- Pattern editing [submenu]
- Consolidate / Bounce to Audio [submenu]
- Color [submenu]
- Edit events in new window
- Edit events in piano roll
- Create automation clip
- Edit automation clip...
- Paste value
- Type in value...

### Rack - unavailable menu placeholders 1
Keywords: rack - unavailable menu placeholders 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Edit events [not implemented]
- Copy value [not implemented]
- Set... [not implemented]
- Advanced looping [not implemented]

### Rack Controls - menu labels 1
Keywords: rack controls - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Route selected channels to consecutive mixer tracks...

### Channel Strip mixer - control hints 1
Keywords: channel strip mixer - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Slide to the next complete plugin section

### Channel Strip mixer - menu labels 1
Keywords: channel strip mixer - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Choose channel strip [submenu]
- Fill all insert channels with [submenu]
- Other resizable strips (unsupported) [submenu]
- Use an existing effect [submenu]
- Channel width [submenu]

### Record - menu labels 1
Keywords: record - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Audio, into the playlist as an audio clip [submenu]
- Audio, into the audio editor
- Don't ask again
- Merge
- Replace
- Cancel
- Respect recording filter
- Only when recording
- Automation mode [submenu]
- Touch (release returns to automation)
- Latch (keep writing until Stop)
- Record as separate clips

### Record Lead - menu labels 1
Keywords: record lead - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Off
- Count-in (clicks only)
- Pre-roll (play before recording)

### Remote - menu labels 1
Keywords: remote - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Link to controller...
- Override global link...
- Remove link

### Route Picker - menu labels 1
Keywords: route picker - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Route to Master
- Choose Mixer track...
- Show current Mixer track
- Drag up/down to change Mixer track

### Theme Library - menu labels 1
Keywords: theme library - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Duplicate as my theme
- Rename...
- Delete...
- Show in folder

### Arpeggiate view - control hints 1
Keywords: arpeggiate view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Close (puts the notes back)
- Group arpeggiated notes

### Browser view - control hints 1
Keywords: browser view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Move here to show the browser
- Browser options
- Edit parameter value
- Plugin presets
- Drag to scroll samples
- Search the browser (Enter = find next, Esc = clear)
- Browser: drag to resize, double-click for the default width

### Cabledebug view - control hints 1
Keywords: cabledebug view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Cable physics debug panel: drag a slider, right-click resets it

### Channel Win view - control hints 1
Keywords: channel win view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Drag to move

### Dialog view - control hints 1
Keywords: dialog view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- AI assistant: click to speak. Microphone is off while idle.
- Dismiss the assistant orb

### Menus view - control hints 1
Keywords: menus view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Select pattern

### Menus view - menu labels 1
Keywords: menus view - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- MIDI file...
- Project import report...
- Resolve imported files...
- Select pattern [submenu]

### Midi Form view - control hints 1
Keywords: midi form view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Import the file with these settings (Enter)

### Midi Page view - control hints 1
Keywords: midi page view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Clear this MIDI binding
- Midi master clock sync offset (milliseconds; wheel or drag, right-click: reset)
- Midi external clock sync offset (milliseconds; wheel or drag, right-click: reset)
- Edit mapping curve
- Device whose MIDI clock drives playback
- Scroll MIDI devices
- No device selected
- Close (drops the changes)

### Traditional view - control hints 1
Keywords: traditional view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Add more tracks
- Mixer scroll area
- Mute / solo
- Reverse polarity
- Swap left & right channels
- Enable / bypass track effects
- Arm / disarm disk recording
- Traditional mixer layout

### Vintage Compact view - control hints 1
Keywords: vintage compact view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Mixer scroll area
- Mute / solo
- Scroll effect slots
- Effect slot options
- Enable / bypass slot
- Add another effect slot
- Audio input / recording source

### Mixer view - control hints 1
Keywords: mixer view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Pause here to insert a mixer track
- Insert an empty mixer track here
- Supported Waves channel strips have been tested. Compatible resizable UAD channel strips are supported but untested.
- Add more tracks
- Mixer scroll area
- Mixer options
- Add a new track after the selected track
- Mute / solo
- Reverse polarity
- Swap left & right channels
- Drag to resize this inspector section
- Add effect slot
- Scroll insert effects
- Open effect or choose a plugin
- Insert effect options
- Enable / bypass effect

### Mixer view - control hints 2
Keywords: mixer view - control hints 2 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Edit sends and routing
- Choose a mixer channel, including Master
- Stereo output level
- Drag to move
- Scroll effect slots
- Mute / solo  (ctrl)0..9
- Add another effect slot (up to 63)
- Send this channel to an additional pair on your audio interface

### Mod view - control hints 1
Keywords: mod view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Detach / attach window - move freely between monitors
- Drag to move
- Ok with this ticked turns this message off (Manage messages)

### Mondebug view - control hints 1
Keywords: mondebug view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Monitor debug panel: drag a slider, right-click resets it

### Patterns view - control hints 1
Keywords: patterns view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Edit or clear this swatch
- Name suggestions
- Use this name
- Set the colour

### Perf view - control hints 1
Keywords: perf view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Drag to move
- Close (the measurements are reset)
- Find: show only the plugins whose name, patch or place has this text (Esc clears)
- Double-click: open the plugin's window

### Piano Roll view - control hints 1
Keywords: piano roll view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Piano roll: click to add notes; Alt+wheel edits note velocity (empty space: row height); right-click empty space to deselect with Pencil or Brush
- Click to edit this Playlist clip
- Vertical zoom: drag up/down or use the wheel. Right-click resets
- Piano roll options
- Target channel (wheel: channels with notes; right-click: settings)
- Drag: move the play start - right-drag: select a time range
- Event panel: drag to resize, double-click to hide or show
- Event panel: choose which note property to edit
- Event panel: drag to set this property of notes
- Piano roll tools: quantize, chop, glue, strum, arpeggiate, randomize...
- Key and scale: drawing and keyboard snapping
- Chord workspace (Alt+P)
- Undo (Ctrl+Z)
- Draw (P)
- Paint (B): drag to paint notes along the row

### Piano Roll view - control hints 2
Keywords: piano roll view - control hints 2 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Slice (C): drag a line to cut notes
- Delete (D): click or swipe to delete notes
- Mute (T): click or swipe to mute / unmute notes
- Select (E): drag a box to select notes, Ctrl adds
- Stretch toggle: drag note ends to scale selected timing and lengths; drag note bodies to move; right-click empty grid to turn off
- Zoom (Z): drag a box to zoom, right-click zooms out / back
- Playback (Y): drag to hear the notes under the line
- Place Pattern to Playlist (Ctrl+Shift+Enter)

### Piano Dlg view - control hints 1
Keywords: piano dlg view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Close (puts the notes back)
- Keep the spacing of the note starts
- log: End's speed reached along an exponential curve
- What happens to the note lengths
- Range: click or drag, the nearer end follows
- Transpose: click or drag, the key becomes C5
- Scroll the keyboard

### Playlist view - control hints 1
Keywords: playlist view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Stretch audio when resizing
- Stretch (Shift+M): drag a clip edge to stretch its contents
- Clips: patterns, audio clips and automation clips
- Drag to resize the Clip Picker
- Toggle looping for Clip Picker previews
- Clip sources
- Track height: drag up/down or use wheel; right-click or Alt-click resets
- Song position (right-drag: time selection)
- Time marker (click: song position, drag moves, right-click: menu)
- Open track instrument; right-click to choose an instrument
- Choose track instrument
- Mute track (right-click / Ctrl+click: solo)
- Piano Roll; right-click to insert an instrument
- Show / hide automation overlay
- Piano Roll; right-click to insert an instrument on this channel
- Drag tracks onto a folder to add them, or onto another track to reorder

### Playlist view - control hints 2
Keywords: playlist view - control hints 2 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Collapse / expand the group
- Drag to resize this Playlist track
- Type, Enter accepts, Esc cancels

### Project Forms view - control hints 1
Keywords: project forms view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Close without renaming (Esc)

### Studio view - control hints 1
Keywords: studio view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Channel rack
- Drag to resize
- Pattern options
- Drag to resize channel buttons
- Drag between steps to select channels
- Graph editor

### Rack view - control hints 1
Keywords: rack view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Channel rack
- Channel rack - drag to move
- Zipped channel - Alt+U unzips all
- Mute / solo  (shift/ctrl+)0..9
- Global Pitch Follow: toggle without changing the current pitch
- Click: plug-in window; drag: select channels; Ctrl+drag: move rows
- Select (Up/Down)
- Audio waveform: click for sample settings; right-click for channel menu
- Piano roll preview - click to open the Piano roll
- Drag between steps to select channels
- Step - click or drag to paint (left / right)
- Set loop point
- Channel display filter (PgUp/PgDn)
- Drag to resize
- Pattern length: drag or right-click
- Channel options

### Rack view - control hints 2
Keywords: rack view - control hints 2 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Global Pitch Follow for all channels: preserve current pitch
- Drag to resize channel buttons
- Play / pause pattern
- Pattern length
- Loop mode
- Graph editor (Ctrl+K)
- Interactive step sequencer / piano roll overview
- Add a channel
- Click to change
- Write the sequence to the channel's steps (Enter)

### Settings view - control hints 1
Keywords: settings view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Drag Settings
- Close (Esc)
- Pick this colour
- Saturation (left to right) and brightness (bottom to top)
- The colour before: click to go back to it
- Back to the colours from before the picker opened
- Switch the workspace appearance
- Name your new submenu
- Search visibility controls
- Brightens Playlist clips in this theme
- Change the text colour so it reads on every surface
- MP3 music genres
- Remove this folder
- Listed when a plugin is found in several formats
- Sort by this column
- Find: type to keep the plugins whose name, vendor or file contain the text (Esc clears)

### Settings view - control hints 2
Keywords: settings view - control hints 2 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Display PPI: drag, or turn the mouse wheel

### Strum view - control hints 1
Keywords: strum view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Close (puts the notes back)

### Tapper view - control hints 1
Keywords: tapper view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Tap me, or press any key on your MIDI keyboard
- Nudge tempo to align time (Left / pitch bend)
- Nudge tempo to align time (Right / pitch bend)
- Apply tempo: tapped tempos set the project tempo

### Toolbar view - control hints 1
Keywords: toolbar view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Open Tesla audio editor
- Donate through Michael Lane's PayPal.me page
- Report a bug or send feedback: review technical details before submitting
- Toolbar options · some controls are hidden at this width
- Logo - drag onto the palette to take it off
- About ML Studio
- Pattern mode: play the current pattern
- Song mode: play the playlist
- Tempo (drag up/down: the whole number, or the decimal place the drag starts on)
- Oscilloscope / spectrum analyzer (click to switch)
- Output level (peak L / R)
- Song position: the playlist is empty
- New pattern (find first / next empty)
- Pattern options
- Row - drag onto the other row to swap them
- The toolbar presets, Save preset as... and Delete preset

### Toolbar view - control hints 2
Keywords: toolbar view - control hints 2 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Done: the toolbar keeps this layout

### Typein view - control hints 1
Keywords: typein view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Type, Enter accepts, Esc cancels
- Close (Esc)

### Vintage Toolbar view - control hints 1
Keywords: vintage toolbar view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Pattern options
- Live output waveform and stereo peak levels
- Pattern mode
- Song mode
- Tempo: drag whole number or decimal digit; right-click for options

### View - control hints 1
Keywords: view - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Drag to scroll pitch; wheel over notes to scroll, Ctrl+Shift+wheel to zoom pitch
- Vocal editor
- Drag value up/down; Alt for fine changes, Alt-click resets
- Wheel: pitch scroll · Shift: time scroll · Ctrl: time zoom · Ctrl+Shift: pitch zoom · drag empty space to select
- Drag empty space to select; choose Draw to paint pitch
- Drag pitch scrollbar
- Drag time navigator; the Playlist timeline stays aligned
- Vocal tools

### Warp - control hints 1
Keywords: warp - control hints 1 reference controls options settings
These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.

- Audio timing editor
- Retry rendering audio edits
- Close timing editor

### Workspace Layout - menu labels 1
Keywords: workspace layout - menu labels 1 reference controls options settings
These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.

- Assign layout hotkey [submenu]
- Delete saved layout [submenu]

### Configurable command shortcuts - exact defaults
Keywords: configurable command shortcuts - exact defaults reference controls options settings
Settings > General > Input > Manage shortcuts changes these defaults. Apply keeps the changes; clearing a command disables its binding. A reassigned default does not trigger the old action. Ctrl means Command on macOS.

- Play / stop: Space
- Play / pause: Ctrl+Space
- Save project: Ctrl+S
- Save project as: Ctrl+Shift+S
- Open project: Ctrl+O
- Undo: Ctrl+Z
- Redo: Ctrl+Shift+Z
- Playlist: F5
- Channel rack: F6
- Piano roll: F7
- Mixer: F9
- Metronome: Ctrl+M
- Typing piano keyboard: Ctrl+T

### Citrus - parameter names 1
Keywords: citrus - parameter names 1 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Mod X
- Mod Y

### ML Delay - parameter names 1
Keywords: ml delay - parameter names 1 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Input wet
- Delay type
- Tempo sync
- Keep pitch
- Time
- Delay time R/L offset
- Time smooth
- Stereo spread
- Modulation rate
- Modulation time
- Modulation cutoff
- Diffusion level
- Diffusion spread
- Distortion type
- Feedback level
- Filter mode
- Feedback cutoff
- Feedback resonance
- Distortion
- Distortion knee

### ML Delay - parameter names 2
Keywords: ml delay - parameter names 2 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Distortion symmetry
- Rate reduction
- Bits reduction
- Output wet
- Output tone
- Output dry

### ML Limiter - parameter names 1
Keywords: ml limiter - parameter names 1 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Gain
- Soft saturation threshold
- Limiter ceiling
- Limiter attack time
- Limiter attack curve
- Limiter release time
- Limiter release curve
- Limiter peak window
- Comp threshold
- Comp ratio
- Comp knee
- Comp attack time
- Comp release time
- Comp curve
- Comp RMS window
- Noise gain
- Noise threshold
- Noise release time

### ML Reverb - parameter names 1
Keywords: ml reverb - parameter names 1 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Low cut
- High cut
- Predelay
- Room size
- Diffusion
- Decay time
- High damping
- Bass multiplier
- Crossover
- Stereo separation
- Dry level
- Early reflection level
- Wet level
- Mod Speed
- Mod Depth

### ML Sampler - parameter names 1
Keywords: ml sampler - parameter names 1 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Keep on disk
- Resample
- Load regions
- Load slice markers
- Loop start position
- Loop end position
- Use loop points
- Ping pong loop
- Pitch shift
- Time stretch (multiplicator)
- Time stretch (time)
- Remove DC offset
- Normalize
- Reverse
- Reverse polarity
- Fade stereo
- Swap stereo
- Clipping
- Stereo delay
- Loop points follow a reversal

### ML Sampler - parameter names 2
Keywords: ml sampler - parameter names 2 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Root note
- Fine tuning
- Affected by the main pitch
- Add root note to key
- Zone low key
- Zone high key

### ML Chorus - parameter names 1
Keywords: ml chorus - parameter names 1 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Delay
- Depth
- Stereo
- LFO 1 speed
- LFO 2 speed
- LFO 3 speed
- LFO 1 wave
- LFO 2 wave
- LFO 3 wave
- Cross type
- Cross cutoff
- Wet only

### ML Stereo Shaper - parameter names 1
Keywords: ml stereo shaper - parameter names 1 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Right into left
- Left
- Right
- Left into right
- Delay
- Phase
- Effects position

### ML Tesla - parameter names 1
Keywords: ml tesla - parameter names 1 reference controls options settings
These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.

- Play / pause
- Record

<!-- source-reference-end -->

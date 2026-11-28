+ PowerWhittle's post for hobby routers on grbl (grbl 1.1, grblHAL), millimetres, tools changed
+ by hand: grbl refuses M6, so a tool change stops the spindle and pauses (M0) for the
+ new tool, to be resumed from the sender once it is in and zeroed.
+ Rotary: Y wrapped onto A, for a cylinder along X; feeds in inverse time (G93). Needs an A axis (grblHAL built with 4 or more axes; standard GRBL 1.1 has none).
+ Made from this controller's Arcs post: change the two together.

+ History
+ Who        When       What
+ ========== ========== ==========================================
+ PowerWhittle  2026-10-27 First version: arcs, tool change by hand (M0)
+ PowerWhittle  2026-10-31 Dwell written as G4

POST_NAME = "PowerWhittle grblHAL Rotary A, cylinder along X, G93 (mm) (*.nc)"
FILE_EXTENSION = "nc"
UNITS = "MM"
LINE_ENDING = "[10]"
+ round brackets start G-code comments, so names use curly ones
SUBSTITUTE = "({)}"
ROTARY_WRAP_Y = "A"
INVERSE_TIME_MODE = YES

VAR SPINDLE_SPEED = [S|A|S|1.0]
VAR INVERSE_TIME = [FI|A|F|1.3]
VAR X_POSITION = [X|C|X|1.3]
VAR Y_POSITION = [Y|C|Y|1.3]
VAR Z_POSITION = [Z|C|Z|1.3]
VAR X_HOME_POSITION = [XH|A|X|1.3]
VAR Y_HOME_POSITION = [YH|A|Y|1.3]
VAR Z_HOME_POSITION = [ZH|A|Z|1.3]
VAR SAFE_Z_HEIGHT = [SAFEZ|A|Z|1.3]
VAR TOOL_NUMBER = [T|A|T|1.0]
VAR WRAP_DIAMETER = [WRAP_DIA|A||1.3]
VAR ACROSS_POSITION = [ACROSS|C|Y|1.3]
VAR DWELL_TIME = [DWELL|A|P|1.2]

begin HEADER
"([TP_FILENAME])"
"(Made by [PRODUCT] on [DATE] at [TIME])"
"(Rotary: [WRAP_DIA] mm across, [XLENGTH] x [YLENGTH] unrolled, Z zero on the [Z_ORIGIN])"
"(Toolpaths:)"
"( [TOOLPATHS_OUTPUT])"
"(Tools:)"
"( [TOOLS_USED])"
"G94 G21 G17 G90"
"G0 [ZH]"
"G0 [XH] [YH]"
"(First tool [T]: [TOOLNAME], fitted and zeroed before starting)"
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"
"[S] M3"
"G4 P3"
"G93"

begin INITIAL_RAPID_MOVE
"G0 [X] [Y] [ACROSS]"
"G0 [Z]"

begin RAPID_MOVE
"G0 [X] [Y] [ACROSS] [Z]"

begin FIRST_FEED_MOVE
"G1 [X] [Y] [ACROSS] [Z] [FI]"

begin FEED_MOVE
"G1 [X] [Y] [ACROSS] [Z] [FI]"

begin TOOLCHANGE
"G0 [SAFEZ]"
"G94"
"M5"
"(Change to [T]: [TOOLNAME], zero it, then resume)"
"M0"
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"
"[S] M3"
"G4 P3"
"G93"

begin NEW_SEGMENT
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"

begin FOOTER
"G94"
"G0 [ZH]"
"M5"
"G0 [XH] [YH]"
"M30"

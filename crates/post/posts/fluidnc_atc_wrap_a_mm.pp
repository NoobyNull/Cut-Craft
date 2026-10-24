+ Cut Craft's default post for rotary jobs: grbl / FluidNC, millimetres, automatic tool
+ changer, the axis that goes round the cylinder wrapped onto A, feeds in inverse time (G93).
+ Cut Craft wraps Y for a cylinder along X and X for one along Y.
+ Written for Cut Craft from the documented .pp format (re/SPEC.md).

POST_NAME = "Cut Craft FluidNC ATC Wrap A G93 (mm) (*.nc)"
FILE_EXTENSION = "nc"
UNITS = "MM"
LINE_ENDING = "[10]"
ROTARY_WRAP_Y = "A"
INVERSE_TIME_MODE = YES
+ round brackets start G-code comments, so names use curly ones
SUBSTITUTE = "({)}"

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
"M6 [T]"
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"
"[S] M3"
"G4 P3"
"G93"

begin INITIAL_RAPID_MOVE
"G0 [X] [Y]"
"G0 [Z]"

begin RAPID_MOVE
"G0 [X] [Y] [Z]"

begin FIRST_FEED_MOVE
"G1 [X] [Y] [Z] [FI]"

begin FEED_MOVE
"G1 [X] [Y] [Z] [FI]"

begin TOOLCHANGE
"G94"
"G0 [SAFEZ]"
"M5"
"M6 [T]"
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

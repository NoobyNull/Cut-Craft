+ Cut Craft's post for FluidNC with an automatic tool changer (M6), inches.
+ Rotary: Y wrapped onto A, for a cylinder along X; feeds in inverse time (G93). Needs an A axis.
+ Made by scripts/make_posts.py: edit its template and run it again.

POST_NAME = "Cut Craft FluidNC ATC Wrap Y to A G93 (inch) (*.nc)"
FILE_EXTENSION = "nc"
UNITS = "INCHES"
LINE_ENDING = "[10]"
+ round brackets start G-code comments, so names use curly ones
SUBSTITUTE = "({)}"
ROTARY_WRAP_Y = "A"
INVERSE_TIME_MODE = YES

VAR SPINDLE_SPEED = [S|A|S|1.0]
VAR INVERSE_TIME = [FI|A|F|1.3]
VAR X_POSITION = [X|C|X|1.4]
VAR Y_POSITION = [Y|C|Y|1.4]
VAR Z_POSITION = [Z|C|Z|1.4]
VAR X_HOME_POSITION = [XH|A|X|1.4]
VAR Y_HOME_POSITION = [YH|A|Y|1.4]
VAR Z_HOME_POSITION = [ZH|A|Z|1.4]
VAR SAFE_Z_HEIGHT = [SAFEZ|A|Z|1.4]
VAR TOOL_NUMBER = [T|A|T|1.0]
VAR WRAP_DIAMETER = [WRAP_DIA|A||1.3]

begin HEADER
"([TP_FILENAME])"
"(Made by [PRODUCT] on [DATE] at [TIME])"
"(Rotary: [WRAP_DIA] in across, [XLENGTH] x [YLENGTH] unrolled, Z zero on the [Z_ORIGIN])"
"(Toolpaths:)"
"( [TOOLPATHS_OUTPUT])"
"(Tools:)"
"( [TOOLS_USED])"
"G94 G20 G17 G90"
"G0 [ZH]"
"G0 [XH] [YH]"
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
"G0 [SAFEZ]"
"G94"
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

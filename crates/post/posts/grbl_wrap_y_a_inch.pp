+ Cut Craft's post for hobby routers on grbl (grbl 1.1, grblHAL), inches, tools changed
+ by hand: grbl refuses M6, so a tool change stops the spindle and pauses (M0) for the
+ new tool, to be resumed from the sender once it is in and zeroed.
+ Rotary: Y wrapped onto A, for a cylinder along X; feeds in units a minute. Needs an A axis.
+ Made by scripts/make_posts.py: edit its template and run it again.

POST_NAME = "Cut Craft GRBL Wrap Y to A (inch) (*.nc)"
FILE_EXTENSION = "nc"
UNITS = "INCHES"
LINE_ENDING = "[10]"
+ round brackets start G-code comments, so names use curly ones
SUBSTITUTE = "({)}"
ROTARY_WRAP_Y = "A"

VAR SPINDLE_SPEED = [S|A|S|1.0]
VAR FEED_RATE = [F|C|F|1.2]
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
"(First tool [T]: [TOOLNAME], fitted and zeroed before starting)"
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"
"[S] M3"
"G4 P3"

begin INITIAL_RAPID_MOVE
"G0 [X] [Y]"
"G0 [Z]"

begin RAPID_MOVE
"G0 [X] [Y] [Z]"

begin FIRST_FEED_MOVE
"G1 [X] [Y] [Z] [F]"

begin FEED_MOVE
"G1 [X] [Y] [Z]"

begin TOOLCHANGE
"G0 [SAFEZ]"
"M5"
"(Change to [T]: [TOOLNAME], zero it, then resume)"
"M0"
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"
"[S] M3"
"G4 P3"

begin NEW_SEGMENT
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"

begin FOOTER
"G0 [ZH]"
"M5"
"G0 [XH] [YH]"
"M30"

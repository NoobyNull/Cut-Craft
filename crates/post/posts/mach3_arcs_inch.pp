+ Cut Craft's post for Mach3 (and Mach4), inches, with Mach3's tool change (T M6).
+ Arcs: G2 / G3 with I J from the arc's start.
+ Made by scripts/make_posts.py: edit its template and run it again.

POST_NAME = "Cut Craft Mach3 Arcs (inch) (*.tap)"
FILE_EXTENSION = "tap"
UNITS = "INCHES"
LINE_ENDING = "[10]"
+ round brackets start G-code comments, so names use curly ones
SUBSTITUTE = "({)}"

VAR SPINDLE_SPEED = [S|A|S|1.0]
VAR FEED_RATE = [F|C|F|1.2]
VAR X_POSITION = [X|C|X|1.4]
VAR Y_POSITION = [Y|C|Y|1.4]
VAR Z_POSITION = [Z|C|Z|1.4]
VAR ARC_CENTRE_I_INC_POSITION = [I|A|I|1.4]
VAR ARC_CENTRE_J_INC_POSITION = [J|A|J|1.4]
VAR X_HOME_POSITION = [XH|A|X|1.4]
VAR Y_HOME_POSITION = [YH|A|Y|1.4]
VAR Z_HOME_POSITION = [ZH|A|Z|1.4]
VAR SAFE_Z_HEIGHT = [SAFEZ|A|Z|1.4]
VAR TOOL_NUMBER = [T|A|T|1.0]

begin HEADER
"([TP_FILENAME])"
"(Made by [PRODUCT] on [DATE] at [TIME])"
"(Material: X [XLENGTH] Y [YLENGTH] Z [ZLENGTH], Z zero on the [Z_ORIGIN], XY zero at the [XY_ORIGIN])"
"(Toolpaths:)"
"( [TOOLPATHS_OUTPUT])"
"(Tools:)"
"( [TOOLS_USED])"
"G00 G20 G17 G54 G90 G40 G49 G80"
"G91.1"
"G0 [ZH]"
"G0 [XH] [YH]"
"[T] M6"
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"
"[S] M3"

begin INITIAL_RAPID_MOVE
"G0 [X] [Y]"
"G0 [Z]"

begin RAPID_MOVE
"G0 [X] [Y] [Z]"

begin FIRST_FEED_MOVE
"G1 [X] [Y] [Z] [F]"

begin FEED_MOVE
"G1 [X] [Y] [Z]"

+ arcs: centres relative to their start (I, J)
begin FIRST_CW_ARC_MOVE
"G2 [X] [Y] [Z] [I] [J] [F]"

begin CW_ARC_MOVE
"G2 [X] [Y] [Z] [I] [J]"

begin FIRST_CCW_ARC_MOVE
"G3 [X] [Y] [Z] [I] [J] [F]"

begin CCW_ARC_MOVE
"G3 [X] [Y] [Z] [I] [J]"

begin FIRST_CW_HELICAL_ARC_MOVE
"G2 [X] [Y] [Z] [I] [J] [F]"

begin CW_HELICAL_ARC_MOVE
"G2 [X] [Y] [Z] [I] [J]"

begin FIRST_CCW_HELICAL_ARC_MOVE
"G3 [X] [Y] [Z] [I] [J] [F]"

begin CCW_HELICAL_ARC_MOVE
"G3 [X] [Y] [Z] [I] [J]"

begin TOOLCHANGE
"G0 [SAFEZ]"
"M5"
"[T] M6"
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"
"[S] M3"

begin NEW_SEGMENT
"(Toolpath: [TOOLPATH_NAME], tool: [TOOLNAME])"

begin FOOTER
"G0 [ZH]"
"M5"
"G0 [XH] [YH]"
"M30"

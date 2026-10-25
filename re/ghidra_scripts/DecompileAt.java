// Disassembles and makes a function at an address Ghidra's analysis missed, then prints its
// decompiled C. The project is opened read-only, so nothing is saved. Run like
// DecompileFunction.java with an address:
//   analyzeHeadless re/extracted/ghidra Aspire -process Aspire.exe -noanalysis -readOnly \
//     -scriptPath re/ghidra_scripts -postScript DecompileAt.java 14057f800
import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;

public class DecompileAt extends GhidraScript {
	@Override
	protected void run() throws Exception {
		for (String arg : getScriptArgs()) {
			Address at = toAddr(arg);
			Function function = getFunctionAt(at);
			if (function == null) {
				clearListing(at, at.add(16));
				new DisassembleCommand(at, null, true).applyTo(currentProgram, monitor);
				new CreateFunctionCmd(at).applyTo(currentProgram, monitor);
				function = getFunctionAt(at);
			}
			if (function == null) {
				printerr("could not make a function at " + at);
				continue;
			}
			DecompInterface decompiler = new DecompInterface();
			decompiler.openProgram(currentProgram);
			DecompileResults result = decompiler.decompileFunction(function, 120, monitor);
			println("==== " + function.getName() + " at " + at);
			println(result.decompileCompleted() ? result.getDecompiledFunction().getC() : result.getErrorMessage());
		}
	}
}

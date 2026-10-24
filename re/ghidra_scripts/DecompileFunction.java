// Prints the decompiled C of one function, by name or address, from an already analysed program.
// Run on the saved project without re-analysing it (output goes to the console):
//   analyzeHeadless re/extracted/ghidra Aspire -process Aspire.exe -noanalysis -readOnly \
//     -scriptPath re/ghidra_scripts -postScript DecompileFunction.java <name-or-address>
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;

public class DecompileFunction extends GhidraScript {
	@Override
	protected void run() throws Exception {
		String target = getScriptArgs().length > 0 ? getScriptArgs()[0] : "entry";
		var named = getGlobalFunctions(target);
		Function function = named.isEmpty() ? getFunctionContaining(toAddr(target)) : named.get(0);
		if (function == null) {
			printerr("No function named or at " + target);
			return;
		}
		println(currentProgram.getFunctionManager().getFunctionCount() + " functions analysed");
		DecompInterface decompiler = new DecompInterface();
		decompiler.openProgram(currentProgram);
		DecompileResults result = decompiler.decompileFunction(function, 120, monitor);
		if (!result.decompileCompleted()) {
			printerr("Decompiling " + function.getName() + " failed: " + result.getErrorMessage());
			return;
		}
		println(result.getDecompiledFunction().getC());
	}
}

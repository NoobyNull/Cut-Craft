// Lists the functions that refer to each defined string containing the given text (any case),
// from an already analysed program. Run like DecompileFunction.java:
//   analyzeHeadless re/extracted/ghidra Aspire -process Aspire.exe -noanalysis -readOnly \
//     -scriptPath re/ghidra_scripts -postScript FindStringUses.java <text>
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;

public class FindStringUses extends GhidraScript {
	@Override
	protected void run() throws Exception {
		String wanted = getScriptArgs()[0].toLowerCase();
		for (Data data : currentProgram.getListing().getDefinedData(true)) {
			if (!data.hasStringValue() || data.getValue() == null) {
				continue;
			}
			String text = data.getValue().toString();
			if (!text.toLowerCase().contains(wanted)) {
				continue;
			}
			StringBuilder users = new StringBuilder();
			for (Reference ref : getReferencesTo(data.getAddress())) {
				Function f = getFunctionContaining(ref.getFromAddress());
				users.append(' ').append(f == null ? ref.getFromAddress().toString() : f.getName());
			}
			println(data.getAddress() + " \"" + text + "\" used by:" + users);
		}
	}
}

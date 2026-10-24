// Writes every non-thunk function's decompiled C to <out>/<first 5 hex digits>/<address>_<name>.c
// from an already analysed program, skipping files already written, plus <out>/index.tsv
// (address, name, file). With <worker> <workers> it takes only every <workers>th function, so
// several processes can share the work; re/ghidra_scripts/dump-decompiled runs them.
// The output is Vectric material: write it only under the gitignored re/extracted/.
//   analyzeHeadless re/extracted/ghidra Aspire -process Aspire.exe -noanalysis -readOnly \
//     -scriptPath re/ghidra_scripts -postScript DumpDecompiled.java <out> [<worker> <workers>]
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.decompiler.parallel.DecompilerCallback;
import ghidra.app.decompiler.parallel.ParallelDecompiler;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.util.task.TaskMonitor;
import java.io.PrintWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;

public class DumpDecompiled extends GhidraScript {
	/// Seconds one function may take before it is written as a failure.
	private static final int TIMEOUT = 60;
	/// How often progress is printed, in functions.
	private static final int PROGRESS_EVERY = 5000;

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		Path out = Path.of(args[0]).toAbsolutePath();
		int worker = args.length > 2 ? Integer.parseInt(args[1]) : 0;
		int workers = args.length > 2 ? Integer.parseInt(args[2]) : 1;
		List<Function> all = new ArrayList<>();
		for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
			if (!f.isThunk()) {
				all.add(f);
			}
		}
		List<Function> functions = new ArrayList<>();
		for (int i = worker; i < all.size(); i += workers) {
			if (!Files.exists(out.resolve(fileOf(all.get(i))))) {
				functions.add(all.get(i));
			}
		}
		println(functions.size() + " of " + all.size() + " functions to decompile into " + out);
		AtomicInteger done = new AtomicInteger();
		AtomicInteger failed = new AtomicInteger();
		DecompilerCallback<Void> write = new DecompilerCallback<>(currentProgram, d -> {
			d.toggleCCode(true);
			d.toggleSyntaxTree(false);
		}) {
			@Override
			public Void process(DecompileResults results, TaskMonitor m) throws Exception {
				Function f = results.getFunction();
				Path file = out.resolve(fileOf(f));
				Files.createDirectories(file.getParent());
				String c = results.decompileCompleted()
						? results.getDecompiledFunction().getC()
						: "/* decompiling failed: " + results.getErrorMessage() + " */\n";
				if (!results.decompileCompleted()) {
					failed.incrementAndGet();
				}
				Files.writeString(file, c);
				if (done.incrementAndGet() % PROGRESS_EVERY == 0) {
					println(done.get() + " of " + functions.size() + " decompiled");
				}
				return null;
			}
		};
		write.setTimeout(TIMEOUT);
		try {
			ParallelDecompiler.decompileFunctions(write, functions, monitor);
		}
		finally {
			write.dispose();
		}
		if (worker == 0) {
			Files.createDirectories(out);
			try (PrintWriter index = new PrintWriter(Files.newBufferedWriter(out.resolve("index.tsv")))) {
				for (Function f : all) {
					index.println(f.getEntryPoint() + "\t" + f.getName(true) + "\t" + fileOf(f));
				}
			}
		}
		println(done.get() + " decompiled, " + failed.get() + " failed; index at " + out.resolve("index.tsv"));
	}

	private static String fileOf(Function f) {
		String address = f.getEntryPoint().toString();
		String name = f.getName().replaceAll("[^A-Za-z0-9_.-]", "_");
		if (name.length() > 80) {
			name = name.substring(0, 80);
		}
		return address.substring(0, Math.min(5, address.length())) + "/" + address + "_" + name + ".c";
	}
}

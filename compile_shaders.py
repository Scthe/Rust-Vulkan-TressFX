import re
import sys
import subprocess
from os import listdir
from os.path import isfile, join, basename, dirname

SHADER_SRC_DIR = "./assets/shaders"
SHADER_OUT_DIR = "./assets/shaders-compiled"
SHADER_IMPORT = "//@import"
COMPILER_ERROR_REGEX = "^.*?:(.*?):\W*(.*?):(.*)$"
PRINT_TRACE = False

class Colors:
	BLACK   = '\033[0;{}m'.format(30)
	RED     = '\033[0;{}m'.format(31)
	GREEN   = '\033[0;{}m'.format(32)
	YELLOW  = '\033[0;{}m'.format(33)
	BLUE    = '\033[0;{}m'.format(34)
	MAGENTA = '\033[0;{}m'.format(35)
	CYAN    = '\033[0;{}m'.format(36)
	WHITE   = '\033[0;{}m'.format(37)

def trace(line):
	if PRINT_TRACE:
		print(line)

def is_valid_shader_file(path, allow_underscore=False):
	# ends with "glsl", does not start with "_"
	name = basename(path)
	undsc_failed = name.startswith("_") and (not allow_underscore)
	return isfile(path) and path.endswith(".glsl") and not undsc_failed

# SECTION: LIST SHADER FILES
def list_shader_files(path):
	content_filenames = listdir(path)
	content_files = [join(path, f) for f in content_filenames]
	return [f for f in content_files if is_valid_shader_file(f)]

# SECTION: PROCESS SHADER FILE FOR IMPORTS
def print_import_stack(import_stack):
		print("Import stack:" + "\n\t".join(import_stack))

def get_path_of_imported_file(current_file, import_line, import_stack):
	start_idx = len(SHADER_IMPORT) + 1
	end_idx = import_line.find(";", start_idx)
	imported_filename = import_line[start_idx : end_idx]
	imported_filename = imported_filename.startswith("./") and imported_filename[2:] or imported_filename
	imported_filename = imported_filename.endswith(".glsl") and imported_filename or f"{imported_filename}.glsl"
	imported_filepath = join(dirname(current_file), imported_filename)
	
	if not is_valid_shader_file(imported_filepath, True):
		print(f"Unable to process '{import_line}' in '{current_file}'. Resolved file: '{imported_filepath}'")
		print_import_stack(import_stack)
		sys.exit(1)
	
	return imported_filepath

def process_shader_file(path, import_stack=[]):
	buffer = []
	if path in import_stack:
		return buffer # already processed

	import_stack = import_stack + [path]
	if len(import_stack) == 4:
		print(f"Unable to import '{path}', tried to import too deep. Do you have circular imports?")
		print_import_stack(import_stack)
		sys.exit(1)

	with open(path) as file:
		for line in file:
			is_include_line = line.lstrip().startswith(SHADER_IMPORT)
			# print(line)
			if is_include_line:
				line = line.rstrip()
				trace(f"\tFound import '{line}'")
				imported_filepath = get_path_of_imported_file(path, line.lstrip(), import_stack)
				imported_file_content = process_shader_file(imported_filepath, import_stack)
				buffer.append(f"// START IMPORT: '{imported_filepath}'\n")
				buffer.extend(imported_file_content)
				buffer.append(f"\n// END IMPORT: '{imported_filepath}'\n")
			else:
				buffer.append(line)
	return buffer

def write_processed_shader_file(path, lines):
	out_path = path.replace(SHADER_SRC_DIR, SHADER_OUT_DIR)
	trace(f"\tWritting to {out_path}")
	
	with open(out_path, 'w') as f:
		for line in lines:
			f.write(line)
	
	return out_path

# SECTION: COMPILE
def print_compile_error_line(error_line, shader_lines):
	print(error_line)
	matches = re.search(COMPILER_ERROR_REGEX, error_line, re.IGNORECASE)
	line_printed = False

	if matches:
		try:
			line_no, level, msg = int(matches.group(1)), matches.group(2), matches.group(3)
			# print(line_no, level, msg)
			line = shader_lines[line_no - 1].strip()
			is_warn = level == 'warning'
			col = Colors.YELLOW if is_warn else Colors.RED
			level_str = 'Warn' if is_warn else 'Error'
			print('{}[L{}] {}: {}'.format(col, line_no, level_str, msg))
			print('{}   > {}{}'.format(Colors.CYAN, line, Colors.WHITE))
			line_printed = True
		except:
			pass
	
	if not line_printed:
		print(error_line)

def compile_shader(path, shader_lines):
	# glslc.exe -O -fshader-stage=frag $< -o $@
	shader_stage = None
	if path.endswith(".vert.glsl"):
		shader_stage = "vert"
	if path.endswith(".frag.glsl"):
		shader_stage = "frag"
	if shader_stage is None:
		print(f"Unable to guess shader type from filepath '{path}'")
		sys.exit(1)
	pass

	out_path = path.replace(".glsl", ".spv")
	trace(f"\tCompiling {shader_stage} shader to '{out_path}'")
	result = subprocess.run(
		["glslc.exe", "-O", f"-fshader-stage={shader_stage}", path, "-o", out_path],
		capture_output=True, text=True
	)

	if result.returncode == 0:
		trace(f"\tSuccessully compiled to '{out_path}'")
	else:
		print(f"Error compiling '{path}'")
		error_lines = result.stderr.split("\n")
		for error_line in error_lines:
			print_compile_error_line(error_line, shader_lines)
		sys.exit(1)


#
# MAIN
# 

shader_files = list_shader_files(SHADER_SRC_DIR)
# shader_files = ["./assets/shaders/forward.frag.glsl"]
# shader_files = ["./assets/shaders/forward.vert.glsl"]
for shader_file in shader_files:
	print(f"Processing '{shader_file}'")
	lines = process_shader_file(shader_file)
	processed_shader_path = write_processed_shader_file(shader_file, lines)
	compile_shader(processed_shader_path, lines)
# $@ - left side of ':'
# $^ - right side of ':'
# $< - first of dependencies
#
#  glslc.exe -O -fshader-stage=vert src/shaders/triangle.vert.glsl -o vert.spv
# TODO glslangValidator.exe
# TODO spirv-dis
# TODO spirv-reflect


build_shaders:
	@python compile_shaders.py

clean:
	@rm target/debug/rs-tressfx.exe,\
		target/debug/rs-tressfx.d,\
		target/debug/rs_tressfx.pdb,\
		$(SHADER_OUT_DIR)/*.spv;\
		echo CLEANED

# run: clean build_shaders
run: build_shaders
	cargo run

# build: clean build_shaders
build:
	cargo build --release

release: clean build_shaders
	cargo build --release
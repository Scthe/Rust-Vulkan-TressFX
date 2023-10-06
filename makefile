# $@ - left side of ':'
# $^ - right side of ':'
# $< - first of dependencies
#
#  glslc.exe -O -fshader-stage=vert src/shaders/triangle.vert.glsl -o vert.spv
# TODO glslangValidator.exe
# TODO spirv-dis
# TODO spirv-reflect


SHADER_SRC_DIR := assets/shaders
SHADER_OUT_DIR := assets/shaders-compiled
SHADER_SRC_FILES := $(wildcard $(SHADER_SRC_DIR)/*.glsl)
SHADER_OUT_FILES := $(patsubst $(SHADER_SRC_DIR)/%.glsl,$(SHADER_OUT_DIR)/%.spv,$(SHADER_SRC_FILES))

build_shaders: $(SHADER_OUT_FILES)
	@echo Shaders compiled succesfully

$(SHADER_OUT_DIR)/%.vert.spv: $(SHADER_SRC_DIR)/%.vert.glsl
	glslc.exe -O -fshader-stage=vert $< -o $@

$(SHADER_OUT_DIR)/%.frag.spv: $(SHADER_SRC_DIR)/%.frag.glsl
	glslc.exe -O -fshader-stage=frag $< -o $@


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
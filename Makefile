DIR = ./assets/shaders
GLSL_FLAGS = --target-env vulkan1.3 --spirv-val
SHADERS = triangle.frag triangle.vert shader.comp
GLSL = glslang

all: $(SHADERS:%=$(DIR)/%.spv)

$(DIR)/%.spv: $(DIR)/%
	$(GLSL) $(GLSL_FLAGS) $< -o $@

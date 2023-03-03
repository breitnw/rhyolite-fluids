#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_color;

layout(set = 0, binding = 1) uniform UAmbientLightData {
    vec4 color;
    float intensity;
} light;

layout(location = 0) out vec4 f_color;

void main() {
    vec3 ambient_color = light.intensity * light.color.rgb;
    vec3 combined_color = ambient_color * subpassLoad(u_color).rgb;
    f_color = vec4(combined_color, 1.0);
}
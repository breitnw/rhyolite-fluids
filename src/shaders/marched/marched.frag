#version 450

layout(location = 0) in vec2 uv;

layout(set = 1, binding = 0) uniform marching_data {
    float time;
} data;

layout(location = 0) out vec4 out_color;

float smin(float a, float b, float k) {
    float h = clamp(0.5 + 0.5*(a-b)/k, 0.0, 1.0);
    return mix(a, b, h) - k*h*(1.0-h);
}

float distance_from_sphere(in vec3 p, in vec3 c, float r) {
    return length(p-c) - r;
}

float map_the_world(in vec3 p) {
    float displacement = sin(5.0 * p.x + data.time * 0.5) * sin(5.0 * p.y + data.time * 2.0) * sin(5.0 * p.z + data.time) * 0.25;
    float sphere_0 = distance_from_sphere(p, vec3(0.0), 1.0);
    float sphere_1 = distance_from_sphere(p, vec3(sin(data.time * 0.6) * 5.0, 0.0, 0.0), 1.0);
    return smin(sphere_0 + displacement, sphere_1, 2.0);
}

vec3 get_normal(in vec3 p) {
    const vec3 step = vec3(0.001, 0.0, 0.0);
    float gradient_x = map_the_world(p + step.xyy) - map_the_world(p - step.xyy);
    float gradient_y = map_the_world(p + step.yxy) - map_the_world(p - step.yxy);
    float gradient_z = map_the_world(p + step.yyx) - map_the_world(p - step.yyx);
    vec3 normal = vec3(gradient_x, gradient_y, gradient_z);
    
    return normalize(normal);
}


vec3 ray_march(in vec3 ro, in vec3 rd) {
    float total_distance_traveled = 0.0;
    const int NUMBER_OF_STEPS = 100;
    const float MINIMUM_HIT_DISTANCE = 0.001;
    const float MAXIMUM_TRACE_DISTANCE = 1000.0;
    
    for (int i = 0; i < NUMBER_OF_STEPS; i++) {
        vec3 current_position = ro + total_distance_traveled * rd;
        float distance_to_closest = map_the_world(current_position);
        
        if (distance_to_closest < MINIMUM_HIT_DISTANCE) {
            const vec3 light_pos = vec3(2.0, -5.0, 3.0);
            vec3 direction_to_light = normalize(current_position - light_pos);

            vec3 normal = get_normal(current_position);

            float diffuse_intensity = max(0.0, dot(normal, direction_to_light));
            
            return vec3(1.0, 0.0, 0.0) * diffuse_intensity;
        }
        
        if (total_distance_traveled > MAXIMUM_TRACE_DISTANCE) {
            break;
        }
        
        total_distance_traveled += distance_to_closest;
    }
    return vec3(0.0);
}

void main() {    
    vec3 cam_pos = vec3(0.0, 0.0, -5.0);
    vec3 ro = cam_pos;
    vec3 rd = normalize(vec3(uv, 1.0));
    
    vec3 shaded_color = ray_march(ro, rd);
    
    out_color = vec4(shaded_color, 1.0);
}
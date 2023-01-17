#version 450

layout(location = 0) in vec2 uv;

layout(set = 0, binding = 0) uniform VP_Data {
    mat4 view;
    mat4 projection;
} vp_uniforms;

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

vec3 phong(in vec3 frag_pos, in vec3 light_pos, in vec3 cam_pos) {
    const float specular_intensity = 1.0;
    const float specular_shininess = 64.0;
    const vec3 light_color = vec3(1.0, 0.0, 0.0);
    const float light_intensity = 1.0;

    vec3 light_dir = light_pos - frag_pos;
    float dist_squared = pow(length(light_dir), 2);
    light_dir = normalize(light_dir);

    vec3 normal = get_normal(frag_pos);

    float lambertian = max(dot(normal, light_dir), 0.0);
    float specular = 0.0;

    if (lambertian > 0.0) {
        vec3 view_dir = normalize(cam_pos - frag_pos);
        vec3 reflect_dir = reflect(-light_dir, normal);
        float spec_angle = max(dot(reflect_dir, view_dir), 0.0);
        specular = pow(spec_angle, specular_shininess);
    }

    vec3 out_color = (lambertian * light_color + specular) * light_intensity / dist_squared;

    return 1.0 * out_color;
}

vec3 ray_march(in vec3 ro, in vec3 rd) {
    float distance_traveled = 0.0;
    const int NUMBER_OF_STEPS = 100;
    const float MINIMUM_HIT_DISTANCE = 0.01;
    const float MAXIMUM_TRACE_DISTANCE = 50.0;
    
    for (int i = 0; i < NUMBER_OF_STEPS; i++) {
        vec3 current_position = ro + distance_traveled * rd;
        float distance_to_closest = map_the_world(current_position);
        
        if (distance_to_closest < MINIMUM_HIT_DISTANCE) {
            const vec3 light_pos = vec3(2.0, -5.0, 3.0);
            return phong(current_position, light_pos, ro);

            // vec3 direction_to_light = normalize(light_pos - current_position);

            // vec3 normal = get_normal(current_position);

            // float diffuse_intensity = max(0.0, dot(normal, direction_to_light));
            
            // return vec3(1.0, 0.0, 0.0) * diffuse_intensity;
        }
        
        if (distance_traveled > MAXIMUM_TRACE_DISTANCE) {
            break;
        }
        
        distance_traveled += distance_to_closest;
    }
    return vec3(0.0);
}

void main() {    
    mat4 view_i = inverse(vp_uniforms.view);
    vec3 cam_pos = vec3(view_i[3][0], view_i[3][1], view_i[3][2]);

    vec3 ro = cam_pos;
    vec3 rd = mat3(vp_uniforms.view) * normalize(vec3(uv, 1.0)); // TODO: rotation is kinda weird here
    rd.z *= -1;
    
    vec3 shaded_color = ray_march(ro, rd);
    
    out_color = vec4(shaded_color, 1.0);
}
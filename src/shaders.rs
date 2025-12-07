pub const SKY_VERTEX_SHADER: &str = r#"
    #version 330 core
    layout (location = 0) in vec3 aPos;

    out vec3 WorldPos;

    uniform mat4 view;
    uniform mat4 projection;

    void main() {
        WorldPos = aPos;
        // Remove translation from view matrix for skybox (it stays with player)
        mat4 viewRot = mat4(mat3(view)); 
        vec4 pos = projection * viewRot * vec4(aPos, 1.0);
        // Force depth to max (w) to render behind everything if we used depth test, 
        // but we will just draw it first.
        gl_Position = pos.xyww;
    }
"#;

pub const SKY_FRAGMENT_SHADER: &str = r#"
    #version 330 core
    in vec3 WorldPos;
    out vec4 FragColor;

    uniform vec3 uSunDir;
    uniform vec3 uMoonDir; // Explicit moon direction
    uniform float uTime;
    uniform vec3 uCameraPos; // Added camera position for world-space clouds

    // --- Noise Functions ---
    float hash(vec3 p) {
        p  = fract( p*0.3183099+.1 );
        p *= 17.0;
        return fract( p.x*p.y*p.z*(p.x+p.y+p.z) );
    }

    float noise(vec3 x) {
        vec3 i = floor(x);
        vec3 f = fract(x);
        f = f*f*(3.0-2.0*f);
        return mix(mix(mix( hash(i+vec3(0,0,0)), 
                            hash(i+vec3(1,0,0)),f.x),
                       mix( hash(i+vec3(0,1,0)), 
                            hash(i+vec3(1,1,0)),f.x),f.y),
                   mix(mix( hash(i+vec3(0,0,1)), 
                            hash(i+vec3(1,0,1)),f.x),
                       mix( hash(i+vec3(0,1,1)), 
                            hash(i+vec3(1,1,1)),f.x),f.y),f.z);
    }

    // FBM with fewer octaves for performance
    float fbm(vec3 x) {
        float v = 0.0;
        float a = 0.5;
        vec3 shift = vec3(100.0);
        for (int i = 0; i < 3; ++i) {
            v += a * noise(x);
            x = x * 2.0 + shift;
            a *= 0.5;
        }
        return v;
    }

    // Cloud density function
    float mapClouds(vec3 p) {
        // Cloud layer bounds
        float cloudBottom = 150.0;
        float cloudTop = 250.0;
        float height = p.y;
        
        // Basic height bounds check
        if (height < cloudBottom || height > cloudTop) return 0.0;
        
        // Height gradient (soft edges top/bottom)
        float hNorm = (height - cloudBottom) / (cloudTop - cloudBottom);
        float hDensity = 1.0 - pow(abs(hNorm - 0.5) * 2.0, 2.0); // Parabola: 0 at edges, 1 in middle
        
        // Wind scrolling
        vec3 q = p - vec3(uTime * 10.0, 0.0, uTime * 4.0);
        
        // Main shape
        float base = fbm(q * 0.015);
        
        // Detail / Erosion
        float detail = fbm(q * 0.05 + vec3(2.3, 4.1, 1.2));
        
        // Combine
        float f = base - detail * 0.3;
        
        // Threshold and Density
        float density = smoothstep(0.4, 0.7, f) * hDensity;
        
        return clamp(density, 0.0, 1.0);
    }
    
    // Henyey-Greenstein phase function
    float hg(float a, float g) {
        float g2 = g*g;
        return (1.0-g2) / (4.0*3.1415*pow(1.0+g2-2.0*g*a, 1.5));
    }

    void main() {
        vec3 viewDir = normalize(WorldPos);
        float sunHeight = uSunDir.y;

        // --- Sky Gradient ---
        vec3 dayZenith = vec3(0.1, 0.4, 0.85);
        vec3 dayHorizon = vec3(0.6, 0.8, 0.95);
        vec3 sunsetZenith = vec3(0.15, 0.1, 0.35);
        vec3 sunsetHorizon = vec3(0.9, 0.45, 0.1);
        vec3 nightZenith = vec3(0.0, 0.0, 0.02);
        vec3 nightHorizon = vec3(0.01, 0.02, 0.08);

        vec3 zenithColor, horizonColor;
        vec3 sunColor = vec3(1.0, 1.0, 0.95);
        float sunIntensity = 1.0;
        float starOpacity = 0.0;
        float cloudBrightness = 1.0;

        if (sunHeight > 0.15) {
            zenithColor = dayZenith;
            horizonColor = dayHorizon;
            starOpacity = 0.0;
            cloudBrightness = 1.2;
        } else if (sunHeight > -0.15) {
            float t = (sunHeight + 0.15) / 0.30;
            zenithColor = mix(sunsetZenith, dayZenith, t);
            horizonColor = mix(sunsetHorizon, dayHorizon, t);
            sunColor = vec3(1.0, 0.8, 0.6); 
            starOpacity = mix(1.0, 0.0, t);
            cloudBrightness = mix(0.9, 1.2, t); 
        } else {
            zenithColor = nightZenith;
            horizonColor = nightHorizon;
            sunIntensity = 0.0;
            starOpacity = 1.0;
            cloudBrightness = 0.15;
        }

        float horizonMix = pow(1.0 - max(viewDir.y, 0.0), 2.5);
        vec3 skyColor = mix(zenithColor, horizonColor, horizonMix);

        // --- Stars ---
        if (starOpacity > 0.0) {
            vec2 starPos = viewDir.xz / (viewDir.y + 1.5) * 200.0; 
            float n = hash(floor(vec3(starPos.x * 1.5, starPos.y * 1.5, 0.0))); 
            float star = step(0.998, n); 
            skyColor += vec3(star * starOpacity * 0.8);
        }

        // --- Sun & Moon ---
        if (sunIntensity > 0.0) {
            float sunDot = dot(viewDir, uSunDir);
            float sunDisk = smoothstep(0.9985, 0.999, sunDot);
            float sunGlow = pow(max(sunDot, 0.0), 400.0) * 0.4;
            skyColor += (sunDisk + sunGlow) * sunColor * sunIntensity;
        }
        // Use explicit moon direction
        float moonDot = dot(viewDir, uMoonDir);
        float moonDisk = smoothstep(0.997, 0.998, moonDot);
        float moonGlow = pow(max(moonDot, 0.0), 200.0) * 0.15;
        skyColor += (moonDisk + moonGlow) * vec3(0.9, 0.95, 1.0) * max(starOpacity, 0.2);

        // --- Volumetric Clouds (World Space) ---
        float cloudBottom = 150.0;
        float cloudTop = 250.0;
        float camY = uCameraPos.y;
        
        float tMin = -1.0;
        float tMax = -1.0;
        
        // Find intersection with cloud layer bounds
        if (camY < cloudBottom) {
            // Below clouds: look up
            if (viewDir.y > 0.0) {
                tMin = (cloudBottom - camY) / viewDir.y;
                tMax = (cloudTop - camY) / viewDir.y;
            }
        } else if (camY > cloudTop) {
            // Above clouds: look down
            if (viewDir.y < 0.0) {
                tMin = (cloudTop - camY) / viewDir.y;
                tMax = (cloudBottom - camY) / viewDir.y;
            }
        } else {
            // Inside clouds
            if (viewDir.y > 0.0) {
                tMin = 0.0;
                tMax = (cloudTop - camY) / viewDir.y;
            } else {
                tMin = 0.0;
                tMax = (cloudBottom - camY) / viewDir.y;
            }
        }

        if (tMin >= 0.0 && tMax > tMin) {
            // Cap distance to avoid marching to infinity
            float maxDist = 800.0;
            if (tMin < maxDist) {
                tMax = min(tMax, maxDist);
                
                int steps = 12; // Optimized
                float dist = tMax - tMin;
                float stepSize = dist / float(steps);
                
                vec3 pos = uCameraPos + viewDir * tMin;
                vec3 rayStep = viewDir * stepSize;
                
                float totalDensity = 0.0;
                vec3 cloudColorAcc = vec3(0.0);
                
                vec3 lightDir = normalize(uSunDir + vec3(0.0, 0.5, 0.0));
                
                // Dither start position to reduce banding (simple hash)
                pos += rayStep * hash(viewDir * uTime);
                
                // Phase function params
                float cosTheta = dot(viewDir, lightDir);
                float phase = max(hg(cosTheta, 0.6), 0.2); 

                for (int i = 0; i < steps; i++) {
                    float den = mapClouds(pos);
                    if (den > 0.01) {
                        // Lighting
                        float shadow = mapClouds(pos + lightDir * 10.0);
                        // Reduced shadow strength to prevent black clouds
                        float lightTransmittance = exp(-shadow * 0.5); 
                        
                        // Powder effect
                        float powder = 1.0 - exp(-den * 2.0);
                        
                        // Lighting components
                        // Force white sun for clouds to avoid brown
                        vec3 cloudSunColor = vec3(1.0, 0.98, 0.95); 
                        
                        // "Silver Lining" Boost: If looking at sun, ignore some shadow
                        float sunGlow = smoothstep(0.8, 1.0, cosTheta);
                        float effectiveTransmittance = mix(lightTransmittance, 1.0, sunGlow * 0.8);

                        vec3 directLight = cloudSunColor * effectiveTransmittance * phase * powder * 3.0;
                        
                        // Ambient: Pure bright white with slight blue tint
                        vec3 ambientLight = vec3(0.9, 0.95, 1.0) * 1.2;
                        
                        // Final scatter color for this sample
                        vec3 scatColor = (directLight + ambientLight) * cloudBrightness;
                        
                        // Accumulate
                        float alpha = den * 0.4;
                        cloudColorAcc += scatColor * alpha * (1.0 - totalDensity);
                        totalDensity += alpha;
                        
                        if (totalDensity >= 0.99) break;
                    }
                    pos += rayStep;
                }
                
                // Blend clouds into sky
                skyColor = mix(skyColor, cloudColorAcc, totalDensity);
                
                // Fog blend (atmospheric perspective)
                float cloudDist = tMin;
                float fogAmt = 1.0 - exp(-cloudDist * 0.002);
                skyColor = mix(skyColor, horizonColor, fogAmt * 0.8);
            }
        }

        FragColor = vec4(skyColor, 1.0);
    }
"#;

pub const SCENE_VERTEX_SHADER: &str = r#"
    #version 330 core
    layout (location = 0) in vec3 aPos;

    out vec3 WorldPos;
    out float HeightRatio;

    uniform mat4 model;
    uniform mat4 view;
    uniform mat4 projection;
    uniform float uMaxHeight;

    void main() {
        vec4 worldPosition = model * vec4(aPos, 1.0);
        WorldPos = worldPosition.xyz;
        // Normalized height for gradient
        HeightRatio = clamp((worldPosition.y + 10.0) / uMaxHeight, 0.0, 1.0);
        gl_Position = projection * view * worldPosition;
    }
"#;

pub const SCENE_FRAGMENT_SHADER: &str = r#"
    #version 330 core
    in vec3 WorldPos;
    in float HeightRatio;
    out vec4 FragColor;

    uniform vec3 uBaseColor;
    uniform vec3 uSunDir;
    uniform vec3 uMoonDir;
    uniform vec3 uCameraPos;

    // Random function for window varying
    float random(vec2 st) {
        return fract(sin(dot(st.xy, vec2(12.9898,78.233))) * 43758.5453123);
    }

    void main() {
        // --- Calculate Face Normal ---
        vec3 xTangent = dFdx(WorldPos);
        vec3 yTangent = dFdy(WorldPos);
        vec3 normal = normalize(cross(xTangent, yTangent));

        // --- Time/Lighting Factors ---
        float sunHeight = uSunDir.y;
        vec3 sunLightColor = vec3(0.0);
        vec3 moonLightColor = vec3(0.0);
        vec3 ambientLight = vec3(0.0);
        float nightFactor = 0.0;

        if (sunHeight > 0.15) {
             // Day
             sunLightColor = vec3(1.0, 0.95, 0.9);
             ambientLight = vec3(0.6, 0.6, 0.65);
             nightFactor = 0.0;
        } else if (sunHeight > -0.15) {
             // Sunset / Sunrise
             sunLightColor = vec3(1.0, 0.6, 0.3);
             ambientLight = vec3(0.4, 0.3, 0.4);
             float t = (sunHeight + 0.15) / 0.30;
             sunLightColor *= t; 
             nightFactor = 1.0 - t;
        } else {
             // Night
             ambientLight = vec3(0.02, 0.02, 0.05);
             nightFactor = 1.0;
        }
        
        // Moon
        if (uMoonDir.y > 0.0) {
            float moonFactor = clamp(uMoonDir.y * 2.0, 0.0, 1.0);
            moonLightColor = vec3(0.2, 0.3, 0.5) * moonFactor;
        }

        // --- Materials ---
        vec3 baseColor = uBaseColor;
        vec3 topColor = vec3(0.95);
        vec3 wallColor = mix(baseColor, topColor, HeightRatio * 0.8);
        
        // Window Logic
        bool isWindow = false;
        if (abs(normal.y) < 0.5) { // Vertical walls
            vec2 gridPos;
            if (abs(normal.x) > 0.5) { gridPos = WorldPos.zy; } 
            else { gridPos = WorldPos.xy; }
            
            vec2 st = gridPos * 1.5; 
            vec2 ipos = floor(st);
            vec2 fpos = fract(st);
            // Window mask
            float w = step(0.3, fpos.x) * step(0.3, fpos.y);
            if (w > 0.5) { isWindow = true; }
            
            // Apply window material (Dark glass)
            if (isWindow) {
                wallColor = vec3(0.1, 0.15, 0.2); // Dark blueish grey glass
            }
        }

        // --- Apply Lighting ---
        float sunDiff = max(dot(normal, uSunDir), 0.0);
        float moonDiff = max(dot(normal, uMoonDir), 0.0);

        // Add specular for glass during day?
        float spec = 0.0;
        if (isWindow && sunHeight > 0.0) {
            vec3 viewDir = normalize(uCameraPos - WorldPos);
            vec3 reflectDir = reflect(-uSunDir, normal);
            spec = pow(max(dot(viewDir, reflectDir), 0.0), 32.0) * 0.8;
        }

        vec3 lighting = wallColor * ambientLight 
                      + wallColor * sunLightColor * sunDiff 
                      + wallColor * moonLightColor * moonDiff
                      + vec3(1.0) * spec * sunLightColor; // Specular highlight

        // --- Window Emission (Night) ---
        if (isWindow && nightFactor > 0.0) {
            vec2 gridPos = (abs(normal.x) > 0.5) ? WorldPos.zy : WorldPos.xy;
            vec2 ipos = floor(gridPos * 1.5);
            float r = random(ipos);
            float lit = step(0.4, r); // 60% lit
            
            vec3 emitColor = vec3(1.0, 0.85, 0.5); // Warm light
            lighting += emitColor * lit * nightFactor * 1.5;
        }

        // Fog
        // Increased density to hide world borders
        float dist = length(WorldPos - uCameraPos);
        
        vec3 dayZenith = vec3(0.1, 0.4, 0.85);
        vec3 dayHorizon = vec3(0.6, 0.8, 0.95);
        vec3 sunsetZenith = vec3(0.15, 0.1, 0.35);
        vec3 sunsetHorizon = vec3(0.9, 0.45, 0.1);
        vec3 nightZenith = vec3(0.0, 0.0, 0.02);
        vec3 nightHorizon = vec3(0.01, 0.02, 0.08);

        vec3 horizonColor;
        if (sunHeight > 0.15) { horizonColor = dayHorizon; }
        else if (sunHeight > -0.15) { 
             float t = (sunHeight + 0.15) / 0.30;
             horizonColor = mix(sunsetHorizon, dayHorizon, t); 
        }
        else { horizonColor = nightHorizon; }

        float fogFactor = 1.0 - exp(-dist * 0.02);
        fogFactor = clamp(fogFactor, 0.0, 1.0);
        
        vec3 finalColor = mix(lighting, horizonColor, fogFactor);

        FragColor = vec4(finalColor, 1.0);
    }
"#;

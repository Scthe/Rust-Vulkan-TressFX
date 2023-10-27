// https://github.com/pyalot/webgl-deferred-irradiance-volumes/blob/master/src/antialias/fxaa3_11_preprocessed.shaderlib
// Honestly, this is just standard Nvidia FXAA 3.11
// Nothing much to add to the code..


vec4 FxaaSampleCol(sampler2D tex, vec2 coords, float bias) {
  // uvec4 value = texture(tex, coords, bias); // as uint [0-255]
  // return vec4(value) / 255.0;
  return texture(tex, coords, bias); // as uint [0-255]
}

float FxaaSampleLuma(sampler2D tex, vec2 coords, float bias) {
  // uint value = texture(tex, coords, bias).a; // as uint [0-255]
  // return float(value) / 255.0;
  return texture(tex, coords, bias).a; // as uint [0-255]
}

vec4 FxaaPixelShader(
    vec2 pos
    ,sampler2D tex
    ,sampler2D lumaTex
    ,vec2 fxaaQualityRcpFrame
    ,float fxaaQualitySubpix
    ,float fxaaQualityEdgeThreshold
    ,float fxaaQualityEdgeThresholdMin
) {
    vec2 posM;
    posM.x = pos.x;
    posM.y = pos.y;
    vec4 rgbyM = FxaaSampleCol(tex, posM, 0.0);
    rgbyM.a = FxaaSampleLuma(lumaTex, posM, 0.0);
    float lumaS = FxaaSampleLuma(lumaTex, posM + (vec2( 0.0, 1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaE = FxaaSampleLuma(lumaTex, posM + (vec2( 1.0, 0.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaN = FxaaSampleLuma(lumaTex, posM + (vec2( 0.0,-1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaW = FxaaSampleLuma(lumaTex, posM + (vec2(-1.0, 0.0) * fxaaQualityRcpFrame.xy), 0.0);

    float maxSM = max(lumaS, rgbyM.y);
    float minSM = min(lumaS, rgbyM.y);
    float maxESM = max(lumaE, maxSM);
    float minESM = min(lumaE, minSM);
    float maxWN = max(lumaN, lumaW);
    float minWN = min(lumaN, lumaW);
    float rangeMax = max(maxWN, maxESM);
    float rangeMin = min(minWN, minESM);
    float rangeMaxScaled = rangeMax * fxaaQualityEdgeThreshold;
    float range = rangeMax - rangeMin;
    float rangeMaxClamped = max(fxaaQualityEdgeThresholdMin, rangeMaxScaled);
    bool earlyExit = range < rangeMaxClamped;
    if(earlyExit) return rgbyM;
    float lumaNW = FxaaSampleLuma(lumaTex, posM + (vec2(-1.0,-1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaSE = FxaaSampleLuma(lumaTex, posM + (vec2( 1.0, 1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaNE = FxaaSampleLuma(lumaTex, posM + (vec2( 1.0,-1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaSW = FxaaSampleLuma(lumaTex, posM + (vec2(-1.0, 1.0) * fxaaQualityRcpFrame.xy), 0.0);

    float lumaNS = lumaN + lumaS;
    float lumaWE = lumaW + lumaE;
    float subpixRcpRange = 1.0/range;
    float subpixNSWE = lumaNS + lumaWE;
    float edgeHorz1 = (-2.0 * rgbyM.y) + lumaNS;
    float edgeVert1 = (-2.0 * rgbyM.y) + lumaWE;
    float lumaNESE = lumaNE + lumaSE;
    float lumaNWNE = lumaNW + lumaNE;
    float edgeHorz2 = (-2.0 * lumaE) + lumaNESE;
    float edgeVert2 = (-2.0 * lumaN) + lumaNWNE;
    float lumaNWSW = lumaNW + lumaSW;
    float lumaSWSE = lumaSW + lumaSE;
    float edgeHorz4 = (abs(edgeHorz1) * 2.0) + abs(edgeHorz2);
    float edgeVert4 = (abs(edgeVert1) * 2.0) + abs(edgeVert2);
    float edgeHorz3 = (-2.0 * lumaW) + lumaNWSW;
    float edgeVert3 = (-2.0 * lumaS) + lumaSWSE;
    float edgeHorz = abs(edgeHorz3) + edgeHorz4;
    float edgeVert = abs(edgeVert3) + edgeVert4;
    float subpixNWSWNESE = lumaNWSW + lumaNESE;
    float lengthSign = fxaaQualityRcpFrame.x;
    bool horzSpan = edgeHorz >= edgeVert;
    float subpixA = subpixNSWE * 2.0 + subpixNWSWNESE;
    if(!horzSpan) lumaN = lumaW;
    if(!horzSpan) lumaS = lumaE;
    if(horzSpan) lengthSign = fxaaQualityRcpFrame.y;
    float subpixB = (subpixA * (1.0/12.0)) - rgbyM.y;
    float gradientN = lumaN - rgbyM.y;
    float gradientS = lumaS - rgbyM.y;
    float lumaNN = lumaN + rgbyM.y;
    float lumaSS = lumaS + rgbyM.y;
    bool pairN = abs(gradientN) >= abs(gradientS);
    float gradient = max(abs(gradientN), abs(gradientS));
    if(pairN) lengthSign = -lengthSign;
    float subpixC = clamp(abs(subpixB) * subpixRcpRange, 0.0, 1.0);
    vec2 posB;
    posB.x = posM.x;
    posB.y = posM.y;
    vec2 offNP;
    offNP.x = (!horzSpan) ? 0.0 : fxaaQualityRcpFrame.x;
    offNP.y = ( horzSpan) ? 0.0 : fxaaQualityRcpFrame.y;
    if(!horzSpan) posB.x += lengthSign * 0.5;
    if( horzSpan) posB.y += lengthSign * 0.5;
    vec2 posN;
    posN.x = posB.x - offNP.x * 1.0;
    posN.y = posB.y - offNP.y * 1.0;
    vec2 posP;
    posP.x = posB.x + offNP.x * 1.0;
    posP.y = posB.y + offNP.y * 1.0;
    float subpixD = ((-2.0)*subpixC) + 3.0;
    float lumaEndN = FxaaSampleLuma(lumaTex, posN, 0.0);
    float subpixE = subpixC * subpixC;
    float lumaEndP = FxaaSampleLuma(lumaTex, posP, 0.0);
    if(!pairN) lumaNN = lumaSS;
    float gradientScaled = gradient * 1.0/4.0;
    float lumaMM = rgbyM.y - lumaNN * 0.5;
    float subpixF = subpixD * subpixE;
    bool lumaMLTZero = lumaMM < 0.0;
    lumaEndN -= lumaNN * 0.5;
    lumaEndP -= lumaNN * 0.5;
    bool doneN = abs(lumaEndN) >= gradientScaled;
    bool doneP = abs(lumaEndP) >= gradientScaled;
    if(!doneN) posN.x -= offNP.x * 1.5;
    if(!doneN) posN.y -= offNP.y * 1.5;
    bool doneNP = (!doneN) || (!doneP);
    if(!doneP) posP.x += offNP.x * 1.5;
    if(!doneP) posP.y += offNP.y * 1.5;
    if(doneNP) {
        if(!doneN) lumaEndN = FxaaSampleLuma(lumaTex, posN.xy, 0.0);
        if(!doneP) lumaEndP = FxaaSampleLuma(lumaTex, posP.xy, 0.0);
        if(!doneN) lumaEndN = lumaEndN - lumaNN * 0.5;
        if(!doneP) lumaEndP = lumaEndP - lumaNN * 0.5;
        doneN = abs(lumaEndN) >= gradientScaled;
        doneP = abs(lumaEndP) >= gradientScaled;
        if(!doneN) posN.x -= offNP.x * 2.0;
        if(!doneN) posN.y -= offNP.y * 2.0;
        doneNP = (!doneN) || (!doneP);
        if(!doneP) posP.x += offNP.x * 2.0;
        if(!doneP) posP.y += offNP.y * 2.0;

        if(doneNP) {
            if(!doneN) lumaEndN = FxaaSampleLuma(lumaTex, posN.xy, 0.0);
            if(!doneP) lumaEndP = FxaaSampleLuma(lumaTex, posP.xy, 0.0);
            if(!doneN) lumaEndN = lumaEndN - lumaNN * 0.5;
            if(!doneP) lumaEndP = lumaEndP - lumaNN * 0.5;
            doneN = abs(lumaEndN) >= gradientScaled;
            doneP = abs(lumaEndP) >= gradientScaled;
            if(!doneN) posN.x -= offNP.x * 4.0;
            if(!doneN) posN.y -= offNP.y * 4.0;
            doneNP = (!doneN) || (!doneP);
            if(!doneP) posP.x += offNP.x * 4.0;
            if(!doneP) posP.y += offNP.y * 4.0;

            if(doneNP) {
                if(!doneN) lumaEndN = FxaaSampleLuma(lumaTex, posN.xy, 0.0);
                if(!doneP) lumaEndP = FxaaSampleLuma(lumaTex, posP.xy, 0.0);
                if(!doneN) lumaEndN = lumaEndN - lumaNN * 0.5;
                if(!doneP) lumaEndP = lumaEndP - lumaNN * 0.5;
                doneN = abs(lumaEndN) >= gradientScaled;
                doneP = abs(lumaEndP) >= gradientScaled;
                if(!doneN) posN.x -= offNP.x * 12.0;
                if(!doneN) posN.y -= offNP.y * 12.0;
                doneNP = (!doneN) || (!doneP);
                if(!doneP) posP.x += offNP.x * 12.0;
                if(!doneP) posP.y += offNP.y * 12.0;
            }

        }

    }
    float dstN = posM.x - posN.x;
    float dstP = posP.x - posM.x;
    if(!horzSpan) dstN = posM.y - posN.y;
    if(!horzSpan) dstP = posP.y - posM.y;
    bool goodSpanN = (lumaEndN < 0.0) != lumaMLTZero;
    float spanLength = (dstP + dstN);
    bool goodSpanP = (lumaEndP < 0.0) != lumaMLTZero;
    float spanLengthRcp = 1.0/spanLength;
    bool directionN = dstN < dstP;
    float dst = min(dstN, dstP);
    bool goodSpan = directionN ? goodSpanN : goodSpanP;
    float subpixG = subpixF * subpixF;
    float pixelOffset = (dst * (-spanLengthRcp)) + 0.5;
    float subpixH = subpixG * fxaaQualitySubpix;
    float pixelOffsetGood = goodSpan ? pixelOffset : 0.0;
    float pixelOffsetSubpix = max(pixelOffsetGood, subpixH);
    if(!horzSpan) posM.x += pixelOffsetSubpix * lengthSign;
    if( horzSpan) posM.y += pixelOffsetSubpix * lengthSign;
    return vec4(FxaaSampleCol(tex, posM, 0.0).xyz, rgbyM.y);

}
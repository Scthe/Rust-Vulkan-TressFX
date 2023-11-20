// https://github.com/pyalot/webgl-deferred-irradiance-volumes/blob/master/src/antialias/fxaa3_11_preprocessed.shaderlib
// Honestly, this is just standard Nvidia FXAA 3.11
// Nothing much to add to the code..

// Extra sources:
//  - https://github.com/GameTechDev/CMAA2/blob/master/Projects/CMAA2/FXAA/Fxaa3_11.h
//  - https://catlikecoding.com/unity/tutorials/advanced-rendering/fxaa/
//  - http://blog.simonrodriguez.fr/articles/2016/07/implementing_fxaa.html


vec4 FxaaSampleCol(usampler2D tex, vec2 coords, float bias) {
  uvec4 value = texture(tex, coords, bias); // as uint [0-255]
  return vec4(value) / 255.0; // float [0-1]
}

float FxaaSampleLuma(usampler2D tex, vec2 coords, float bias) {
  uint value = texture(tex, coords, bias).a; // as uint [0-255]
  return float(value) / 255.0; // float [0-1]
}

float max5(float a, float b, float c, float d, float e) {
  return max(a, max(b, max(c, max(d, e))));
}
float min5(float a, float b, float c, float d, float e) {
  return min(a, min(b, min(c, min(d, e))));
}

vec4 FxaaPixelShader(
    vec2 pos
    ,usampler2D tex
    ,usampler2D lumaTex
    // 1.0/screenSizeInPixels
    ,vec2 fxaaQualityRcpFrame
    // Choose the amount of sub-pixel aliasing removal.
    // This can effect sharpness.
    //   1.00 - upper limit (softer)
    //   0.75 - default amount of filtering
    //   0.50 - lower limit (sharper, less sub-pixel aliasing removal)
    //   0.25 - almost off
    //   0.00 - completely off
    ,float fxaaQualitySubpix
    // relative minimal luma diff (compares contrast to brightest neighbour pixel)
    ,float fxaaQualityEdgeThreshold
    // absolute minimal luma diff
    ,float fxaaQualityEdgeThresholdMin
) {
    vec2 posM;
    posM.x = pos.x;
    posM.y = pos.y;
    // sample middle
    vec4 rgbyM = FxaaSampleCol(tex, posM, 0.0); // .y (green channel) is used as aprox. of luma, cause human eye evolution
    rgbyM.a = FxaaSampleLuma(lumaTex, posM, 0.0);
    // sample 4 pixels around: NEWS - above, right, left, below
    float lumaS = FxaaSampleLuma(lumaTex, posM + (vec2( 0.0, 1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaE = FxaaSampleLuma(lumaTex, posM + (vec2( 1.0, 0.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaN = FxaaSampleLuma(lumaTex, posM + (vec2( 0.0,-1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaW = FxaaSampleLuma(lumaTex, posM + (vec2(-1.0, 0.0) * fxaaQualityRcpFrame.xy), 0.0);

    float rangeMax = max5(lumaN, lumaW, lumaE, lumaS, rgbyM.a); // max of all 5 samples: NEWS+M
    float rangeMin = min5(lumaN, lumaW, lumaE, lumaS, rgbyM.a); // min of all 5 samples: NEWS+M
    float rangeMaxScaled = rangeMax * fxaaQualityEdgeThreshold;
    float range = rangeMax - rangeMin; // contrast between 5 samples
    float rangeMaxClamped = max(fxaaQualityEdgeThresholdMin, rangeMaxScaled);
    // early return if luma diff around pixel is less then threshold
    // threshold can be absolute (`fxaaQualityEdgeThresholdMin`) or relative (`fxaaQualityEdgeThreshold`)
    bool earlyExit = range < rangeMaxClamped;
    if(earlyExit) return rgbyM;

    // sample luma for corners around the middle pixel
    float lumaNW = FxaaSampleLuma(lumaTex, posM + (vec2(-1.0,-1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaSE = FxaaSampleLuma(lumaTex, posM + (vec2( 1.0, 1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaNE = FxaaSampleLuma(lumaTex, posM + (vec2( 1.0,-1.0) * fxaaQualityRcpFrame.xy), 0.0);
    float lumaSW = FxaaSampleLuma(lumaTex, posM + (vec2(-1.0, 1.0) * fxaaQualityRcpFrame.xy), 0.0);

    // factors to detect pixel as part of horizontal/vertical edge
    float lumaNS = lumaN + lumaS;
    float lumaWE = lumaW + lumaE;
    float subpixNSWE = lumaNS + lumaWE;
    // horizontal
    float lumaNESE = lumaNE + lumaSE; // east corners
    float lumaNWSW = lumaNW + lumaSW; // west corners
    float edgeHorz1 = (-2.0 * rgbyM.a) + lumaNS; // contrast in middle colum
    float edgeHorz2 = (-2.0 * lumaE) + lumaNESE; // contrast in east colum
    float edgeHorz3 = (-2.0 * lumaW) + lumaNWSW; // contrast in west colum
    float edgeHorz = abs(edgeHorz3) + (abs(edgeHorz1) * 2.0) + abs(edgeHorz2); // detect horizontal edge
    // vertical
    float lumaNWNE = lumaNW + lumaNE; // north corners
    float lumaSWSE = lumaSW + lumaSE; // south corners
    float edgeVert1 = (-2.0 * rgbyM.a) + lumaWE; // contrast in middle row
    float edgeVert2 = (-2.0 * lumaN) + lumaNWNE; // contrast in north row
    float edgeVert3 = (-2.0 * lumaS) + lumaSWSE; // contrast in south row
    float edgeVert = abs(edgeVert3) + (abs(edgeVert1) * 2.0) + abs(edgeVert2); // detect vertical edge

    // set variables depending on horizontal/vertical edge
    // we will blend perpendicular to the detected edge
    float lengthSign = fxaaQualityRcpFrame.x;
    bool horzSpan = edgeHorz >= edgeVert; // contrast on horizontal line > contrast vertical line
    if(!horzSpan) lumaN = lumaW; // vertical edge: use west, east pixels
    if(!horzSpan) lumaS = lumaE;
    if(horzSpan) lengthSign = fxaaQualityRcpFrame.y; // vertical edge: we will sample left-right so use this value

    // from now on we treat as always horizontal edge (we have just set variables above if it's vertical)
    // detect if in horizontal edge it is top or bottom pixel that contributes more to contrast
    float gradientN = lumaN - rgbyM.a;
    float gradientS = lumaS - rgbyM.a;
    float gradient = max(abs(gradientN), abs(gradientS)); // max contrast between current and N,S pixels
    bool pairN = abs(gradientN) >= abs(gradientS);
    if(pairN) lengthSign = -lengthSign;

    // sample edge pixels in both directions to determine how long the edge is
    vec2 posB;
    posB.x = posM.x;
    posB.y = posM.y;
    vec2 offNP;
    offNP.x = (!horzSpan) ? 0.0 : fxaaQualityRcpFrame.x;
    offNP.y = ( horzSpan) ? 0.0 : fxaaQualityRcpFrame.y;
    if(!horzSpan) posB.x += lengthSign * 0.5;
    if( horzSpan) posB.y += lengthSign * 0.5;
    vec2 posN; // negative
    posN.x = posB.x - offNP.x * 1.0;
    posN.y = posB.y - offNP.y * 1.0;
    vec2 posP; // positive
    posP.x = posB.x + offNP.x * 1.0;
    posP.y = posB.y + offNP.y * 1.0;
    float lumaEndN = FxaaSampleLuma(lumaTex, posN, 0.0);
    float lumaEndP = FxaaSampleLuma(lumaTex, posP, 0.0);
    
    // still sample along the edge..
    float lumaNN = lumaN + rgbyM.a; // sum: luma north + middle
    float lumaSS = lumaS + rgbyM.a;
    if(!pairN) lumaNN = lumaSS;
    float gradientScaled = gradient * 1.0/4.0;
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

    // calc distance to final samples we reached
    float dstN = posM.x - posN.x;
    float dstP = posP.x - posM.x;
    if(!horzSpan) dstN = posM.y - posN.y;
    if(!horzSpan) dstP = posP.y - posM.y;
    float spanLength = (dstP + dstN);
    bool directionN = dstN < dstP;
    float dst = min(dstN, dstP);
    float pixelOffset = (-dst / spanLength) + 0.5;

    // calc blend factor
    float subpixNWSWNESE = lumaNWSW + lumaNESE; // sum: luma all corners
    float subpixA = subpixNSWE * 2.0 + subpixNWSWNESE; // sum: luma of pixels around middle pixel, cardinal dirs weighted 2x more
    // contrast middle pixel vs all 8 pixels around (cardinal dirs weighted 2x, so 4+2*4 = 12)
    // (this is high-pass filter after we subtract `rgbyM.a`)
    float subpixB = (subpixA * (1.0/12.0)) - rgbyM.a; 
    float subpixC = clamp(abs(subpixB) / range, 0.0, 1.0); // contrast_C := (contrast this pixel vs ones around) DIVIDE_BY max contrast of 5 samples
    // I assume smoothstep below?
    float subpixD = ((-2.0)*subpixC) + 3.0;
    float subpixE = subpixC * subpixC;
    float subpixF = subpixD * subpixE;
    float subpixG = subpixF * subpixF;
    float subpixH = subpixG * fxaaQualitySubpix;
    // return vec4(subpixG); // edge detector!


    float lumaMM = rgbyM.a - lumaNN * 0.5; // luma_{this pixel} - luma_{avg this pixel and one near with big contrast}
    bool lumaMLTZero = lumaMM < 0.0;
    bool goodSpanN = (lumaEndN < 0.0) != lumaMLTZero;
    bool goodSpanP = (lumaEndP < 0.0) != lumaMLTZero;
    bool goodSpan = directionN ? goodSpanN : goodSpanP;
    float pixelOffsetGood = goodSpan ? pixelOffset : 0.0;
    float pixelOffsetSubpix = max(pixelOffsetGood, subpixH);
    // sample offsetted `posM` (current pixel and one of it's neighbours)
    if(!horzSpan) posM.x += pixelOffsetSubpix * lengthSign;
    if( horzSpan) posM.y += pixelOffsetSubpix * lengthSign;
    return vec4(FxaaSampleCol(tex, posM, 0.0).xyz, rgbyM.a);

}

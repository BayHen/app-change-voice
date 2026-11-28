"""App-owned isolated data-only RVC checkpoint converter; never a model-supplied script."""
import argparse
import contextlib
import hashlib
import json
import math
from pathlib import Path
import sys
import zipfile

MAX_BYTES = 512 * 1024 * 1024

def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(65536), b''): h.update(block)
    return h.hexdigest()

def load_checkpoint(path, expected):
    if not path.is_file() or not 0 < path.stat().st_size <= MAX_BYTES:
        raise ValueError('Checkpoint must be a regular file no larger than 512 MiB')
    if digest(path) != expected: raise ValueError('Checkpoint changed after selection')
    if not zipfile.is_zipfile(path): raise ValueError('Only modern PyTorch ZIP checkpoints are supported; legacy pickle/tar is rejected')
    with zipfile.ZipFile(path) as archive:
        members=archive.infolist()
        if len(members)>20000 or sum(m.file_size for m in members)>MAX_BYTES:
            raise ValueError('Checkpoint archive exceeds bounded storage limits')
        if any('/code/' in m.filename or m.filename.endswith('/constants.pkl') or '..' in Path(m.filename).parts for m in members):
            raise ValueError('TorchScript and executable checkpoint archives are not accepted')

    import torch
    trusted_owner=expected=='cd4996435d0e9c9f93858a13d9ddf5442a011388478daab1f732e0ac2b2c4020'
    if not trusted_owner and torch.__version__ != '2.10.0+cpu':
        raise ValueError('Secure conversion runtime required: external PTH files require the qualified PyTorch 2.10.0 CPU converter. No checkpoint was deserialized.')
    # No safe_globals, custom unpicklers, weights_only=False, imports from checkpoint,
    # or retries with a less restricted loader are permitted.
    from collections import OrderedDict
    torch.set_num_threads(4)
    cpt = torch.load(path, map_location='cpu', weights_only=True)
    if type(cpt) not in (dict, OrderedDict) or type(cpt.get('config')) not in (list, tuple):
        raise ValueError('Expected an RVC inference checkpoint containing config and weight')
    from collections import OrderedDict
    weights = cpt.get('weight')
    if type(weights) not in (dict, OrderedDict) or not 0 < len(weights) <= 10000:
        raise ValueError('Invalid RVC state dictionary')
    if any(type(k) is not str or type(v) is not torch.Tensor for k, v in weights.items()):
        raise ValueError('Only named tensor weights are accepted')
    if sum(v.numel() * v.element_size() for v in weights.values()) > MAX_BYTES:
        raise ValueError('Checkpoint tensor storage exceeds the conversion limit')
    if any(v.device.type != 'cpu' or not torch.isfinite(v).all().item() for v in weights.values()):
        raise ValueError('Checkpoint contains invalid tensor values')
    cfg = list(cpt['config'])
    if len(cfg) != 18: raise ValueError('Only standard RVC v1/v2 18-field configs are supported')
    def bounded(value, upper):
        return type(value) is int and 0 < value <= upper
    for i, limit in {0:4097,1:8192,2:512,3:512,4:4096,5:16,6:32,7:31,13:2048,15:4096,16:512}.items():
        if not bounded(cfg[i], limit): raise ValueError(f'Unsupported RVC config field {i}')
    if type(cfg[8]) not in (int,float) or not math.isfinite(cfg[8]) or not 0 <= cfg[8] <= 1:
        raise ValueError('Invalid dropout configuration')
    if cfg[9] not in ('1','2'): raise ValueError('Unsupported RVC residual block')
    for i in (10,12,14):
        if type(cfg[i]) not in (list,tuple) or not 0 < len(cfg[i]) <= 8 or not all(bounded(v,128) for v in cfg[i]):
            raise ValueError('Unsupported RVC kernel/upsampling configuration')
    if type(cfg[11]) not in (list,tuple) or not 0 < len(cfg[11]) <= 8 or any(type(row) not in (list,tuple) or not 0 < len(row) <= 8 or not all(bounded(v,128) for v in row) for row in cfg[11]):
        raise ValueError('Unsupported RVC dilation configuration')
    rate = cfg[-1]
    if rate not in (32000,40000,48000): raise ValueError('Unsupported RVC sample rate')
    declared_rate = cpt.get('sr',rate)
    declared_rate = {'32k':32000,'40k':40000,'48k':48000}.get(declared_rate,declared_rate)
    if declared_rate != rate: raise ValueError('Checkpoint sample-rate declarations disagree')
    phone = weights.get('enc_p.emb_phone.weight')
    speaker = weights.get('emb_g.weight')
    if phone is None or phone.ndim != 2 or phone.shape[1] not in (256,768) or speaker is None or speaker.ndim != 2:
        raise ValueError('Unsupported RVC feature or speaker embedding tensors')
    channels = int(phone.shape[1]); version = 'v2' if channels == 768 else 'v1'
    if cpt.get('version',version) != version: raise ValueError('RVC version disagrees with feature tensors')
    if type(cpt.get('f0')) not in (int,bool) or cpt['f0'] not in (0,1):
        raise ValueError('Explicit F0 metadata is required')
    if tuple(speaker.shape) != (cfg[15],cfg[16]): raise ValueError('Speaker embedding shape disagrees with config')
    precision = sorted({str(v.dtype).replace('torch.','') for v in weights.values() if v.is_floating_point()})
    if not set(precision) <= {'float16','float32'}: raise ValueError('Only float16/float32 checkpoints are supported')
    return cpt, {'version':version,'samplingRate':rate,'embChannels':channels,'f0':bool(cpt['f0']),
                 'sourcePrecision':precision,'exportPrecision':'float32','speakerCount':int(speaker.shape[0]),'converterTorchVersion':torch.__version__}

def convert(request, runtime):
    path = Path(request['checkpointPath']).resolve(strict=True)
    cpt, detected = load_checkpoint(path, request['sha256'])
    if request['op'] == 'inspect': return detected
    if 'speakerId' in request:
        speaker = request['speakerId']
        if type(speaker) is not int or not 0 <= speaker < detected['speakerCount']:
            raise ValueError('Requested speaker ID is outside the checkpoint embedding table')
        detected.update(sourceSpeakerCount=detected['speakerCount'], sourceSpeakerId=speaker,
                        speakerCount=1)
        # Export the selected row only. The single-speaker adapter always uses ID zero.
        cpt['weight']['emb_g.weight'] = cpt['weight']['emb_g.weight'][speaker:speaker + 1].clone()
        cpt['config'] = list(cpt['config'])
        cpt['config'][15] = 1
    output = Path(request['outputDirectory']).resolve(strict=True)
    manifest = json.loads((runtime/'runtime-manifest.json').read_text('utf-8'))
    server = (runtime/manifest['serverDirectory']).resolve(strict=True)
    if not server.is_relative_to(runtime): raise ValueError('Invalid managed source directory')
    sys.path.insert(0,str(server))
    import torch
    torch.set_num_threads(4)
    from const import EnumInferenceTypes
    from voice_changer.RVC.onnxExporter.export2onnx import _export2onnx
    kind = (EnumInferenceTypes.pyTorchRVCv2 if detected['f0'] else EnumInferenceTypes.pyTorchRVCv2Nono) if detected['version']=='v2' else (EnumInferenceTypes.pyTorchRVC if detected['f0'] else EnumInferenceTypes.pyTorchRVCNono)
    metadata={'application':'VC_CLIENT','version':'2.1','modelType':kind.value,'samplingRate':detected['samplingRate'],
              'f0':detected['f0'],'embChannels':detected['embChannels'],'embedder':'hubert_base',
              'embOutputLayer':12 if detected['embChannels']==768 else 9,'useFinalProj':detected['embChannels']==256,
              'speakerCount':detected['speakerCount'],
              'sourceCheckpointSha256':request['sha256'],'sourcePrecision':detected['sourcePrecision']}
    if 'sourceSpeakerId' in detected:
        metadata['sourceSpeakerId'] = detected['sourceSpeakerId']
    original_load, original_state, original_export = torch.load, torch.nn.Module.load_state_dict, torch.onnx.export
    def data_only(file,*args,**kwargs):
        if Path(file).resolve(strict=True) != path: raise ValueError('Unexpected checkpoint access')
        return cpt
    def checked_state(module,state,*args,**kwargs):
        result=original_state(module,state,*args,**kwargs)
        # enc_q is a training-only encoder, absent from common inference exports.
        if any(not key.startswith('enc_q.') for key in result.missing_keys) or result.unexpected_keys:
            raise ValueError('Checkpoint is missing inference weights or contains incompatible parameters')
        return result
    def legacy_export(*args,**kwargs):
        kwargs['dynamo']=False
        kwargs['external_data']=False
        return original_export(*args,**kwargs)
    try:
        torch.load=data_only;torch.nn.Module.load_state_dict=checked_state;torch.onnx.export=legacy_export
        _export2onnx(str(path),str(output/'raw.onnx'),str(output/'voice.onnx'),False,metadata)
    finally:
        torch.load=original_load;torch.nn.Module.load_state_dict=original_state;torch.onnx.export=original_export
    import onnx
    model_path=output/'voice.onnx';onnx.checker.check_model(str(model_path))
    return {**detected,'modelPath':str(model_path),'sha256':digest(model_path)}

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--runtime-root',type=Path,required=True);parser.add_argument('--safe-torch-root',type=Path)
    args=parser.parse_args();runtime=args.runtime_root.resolve(strict=True)
    if args.safe_torch_root: sys.path.insert(0,str(args.safe_torch_root.resolve(strict=True)))
    destination=sys.stdout
    for line in sys.stdin:
        if len(line)>256*1024: raise ValueError('Conversion request exceeds limit')
        request=json.loads(line);reply={'protocol':1,'id':request.get('id'),'ok':False}
        if request.get('op')=='shutdown':
            reply['ok']=True;destination.write(json.dumps(reply)+'\n');destination.flush();return
        try:
            if request.get('protocol')!=1 or request.get('op') not in ('inspect','convert'): raise ValueError('Invalid conversion request')
            with contextlib.redirect_stdout(sys.stderr): reply.update(convert(request,runtime))
            reply['ok']=True
        except Exception as error: reply['error']=f'{type(error).__name__}: {str(error)[:1000]}'
        destination.write(json.dumps(reply,allow_nan=False)+'\n');destination.flush()
if __name__=='__main__': main()

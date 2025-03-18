#!/usr/bin/env python3
import os
import shutil
import subprocess
import fnmatch
import json
import statistics

def find_files(directory, pattern):
	ret = []
	for root, dirs, files in os.walk(directory):
		for filename in fnmatch.filter(files, pattern):
			ret.append(os.path.join(root, filename))
	return ret

def append_metrics(ret, metrics):
	for (k1,v1) in metrics.items():
		if not isinstance(v1, dict):
			continue
		if k1 not in ret:
			ret[k1] = {}
		for (k2,v2) in v1.items():
			if k2 not in ret[k1]:
				ret[k1][k2] = []
			if isinstance(v2, list):
				ret[k1][k2] += v2;
			else:
				ret[k1][k2].append(v2)

def analyze_file(filename):
	ret = {}
	with open(filename) as f:
		d = json.load(f)
		if 'spaces' not in d:
			return ret;
		for s1 in d['spaces']:
			if 'spaces' not in s1:
				continue
			for s2 in s1['spaces']:
				if 'metrics' not in s2:
					continue
				metrics = s2['metrics']
				if not isinstance(metrics, dict):
					continue;
				append_metrics(ret, metrics)
	return ret

def analyze_files(filenames):
	ret = {}
	for filename in filenames:
		data = analyze_file(filename)
		append_metrics(ret,data)
	with open('rust-code-analysis.tab', 'w') as output:
		print("#group         \tmethod                   \tminimum\tmedian\tmean\tstd_dev\tmaximum\tcount",file=output)
		for (k1,v1) in ret.items():
			for(k2,v2) in v1.items():
				v2 = sorted(v2)
				minimum = min(v2)
				maximum = max(v2)
				mean = statistics.mean(v2)
				median = statistics.median(v2)
				std_dev = statistics.stdev(v2)
				length = len(v2)
				print(f"{k1:15}\t{k2:25}\t{minimum:.1f}\t{median:.1f}\t{mean:.1f}\t{std_dev:.1f}\t{maximum:.1f}\t{length}",file=output)

def create_json_files(root_path):
	# Ensure directory exists
	if not os.path.exists(root_path):
		os.makedirs(root_path)

	# Clear old data
	shutil.rmtree(root_path)

	# Generate data
	subprocess.call(['rust-code-analysis-cli','-p','src/','-m','-O','json','--pr','-o','rust-code-analysis'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

if __name__ == '__main__':
	root_path = 'rust-code-analysis/src'
	create_json_files(root_path)
	files = find_files(root_path,'*.json')
	analyze_files(files)

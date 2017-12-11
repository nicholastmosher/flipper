#!/usr/local/bin/python2.7

import sys
from elftools.elf.elffile import ELFFile
from elftools.dwarf.enums import ENUM_DW_TAG
from os.path import splitext

class Function:
	def __init__(self, type, name, ret, params):
		self.type = type
		self.name = name
		self.ret = ret
		self.parameters = params
	def __str__(self):
		params = ', '.join(map(str, self.parameters))
		return '%s %s(%s)' % (self.type, self.name, params)

class Parameter:
	def __init__(self, type, name):
		self.type = type
		self.name = name
	def __str__(self):
		return '%s %s' % (self.type, self.name)

def get_die_at_offset(cu, offset):
	die = None
	top = cu.get_top_DIE()
	for d in cu.iter_DIEs():
		if (d.offset == offset): die = d
	return die

def get_parameters_from_die(cu, die):
	parameters = []
	for child in die.iter_children():
		if (child.tag == 'DW_TAG_formal_parameter'):
			name = child.attributes['DW_AT_name'].value
			# print 'n: ' + name
			tdie = get_die_at_offset(cu, child.attributes['DW_AT_type'].value)
			if tdie.tag == 'DW_TAG_typedef':
				tdie = get_die_at_offset(cu, tdie.attributes['DW_AT_type'].value)
			if 'DW_AT_name' in tdie.attributes:
				type = tdie.attributes['DW_AT_name'].value
			else:
				type = 'void *'
			# print 't: ' + type
			p = Parameter(type, name)
			parameters.append(p)
	return parameters

def process_file(filename, package):

	functions = []

	outh = open(package + '.h', 'wb')
	outc = open(package + '.c', 'wb')

	with open(filename, 'rb') as f:
		elffile = ELFFile(f)

		if not elffile.has_dwarf_info():
			print('File has no DWARF info. Compile with -g.')
			return

		dwarfinfo = elffile.get_dwarf_info()

		funcs_addr = 0
		funcs_size = 0
		vars_addr = 0
		vars_size = 0
		for section in elffile.iter_sections():
			if (section.name == '.lf.funcs'):
				funcs_addr = section['sh_addr']
				funcs_size = section['sh_size']
			if (section.name == '.lf.vars'):
				vars_addr = section['sh_addr']
				vars_size = section['sh_size']

		if (funcs_addr == 0): return

		# This iterates through all CUs, even the ones without .lf.funcs section
		i = 0
		for cu in dwarfinfo.iter_CUs():
			top = cu.get_top_DIE()
			for child in top.iter_children():
				if (child.tag == 'DW_TAG_subprogram' and 'DW_AT_low_pc' in child.attributes):
					address = child.attributes['DW_AT_low_pc'].value
					if (address in range(funcs_addr, funcs_addr + funcs_size)):
						name = child.attributes['DW_AT_name'].value
						# print 'n: ' + name
						if 'DW_AT_type' in child.attributes.keys():
							tdie = get_die_at_offset(cu, child.attributes['DW_AT_type'].value)
							while (tdie.tag == 'DW_TAG_typedef'):
								tdie = get_die_at_offset(cu, tdie.attributes['DW_AT_type'].value)
							if 'DW_AT_name' in tdie.attributes:
								type = tdie.attributes['DW_AT_name'].value
								ret = tdie.attributes['DW_AT_byte_size'].value
							elif 'DW_AT_byte_size' in tdie.attributes:
								type = 'void *'
								ret = tdie.attributes['DW_AT_byte_size'].value
							else:
								type = 'void'
								ret = 0x2
						params = get_parameters_from_die(cu, child)
						functions.append(Function(type, name, ret, params))
	h = open('template.h', 'rb')
	htemplate = h.read()
	htemplate = htemplate.replace('PACKAGE', package)
	struct = []
	functs = []
	tags = []
	for f in functions:
		functs.append(str(f) + ';')
		tags.append('_%s_%s' % (package, f.name))
		# print 't:' + f.type
		# print 'n: ' + f.name
		# print 'p: ' + str(f.parameters)
		struct.append('TYPE (* NAME)(PARAMETERS);'.replace('TYPE', f.type).replace('NAME', f.name).replace('PARAMETERS', ', '.join(map(str, f.parameters))))
	htemplate = htemplate.replace('STRUCT', '\t' + '\n\t'.join(struct))
	htemplate = htemplate.replace('FUNCTIONS', '\n'.join(functs))
	htemplate = htemplate.replace('TAGS', ', '.join(tags))
	outh.write(htemplate)

	c = open('template.c', 'rb')
	ctemplate = c.read()
	ctemplate = ctemplate.replace('PACKAGE', package)
	struct = []
	functs = []
	for f in functions:
		struct.append('&%s' % f.name)
		statement = 'lf_invoke(MODULE, FUNCTION, RET, fmr_args(ARGS));'
		args = []
		for p in f.parameters:
			args.append('fmr_infer(ARG)'.replace('ARG', p.name))
		retl = ['fmr_void_t', '', 'fmr_int8_t', 'fmr_int16_t', '', 'fmr_int32_t']
		statement = statement.replace('MODULE', '&_' + package).replace('FUNCTION', '_' + package + '_' + f.name).replace('RET', retl[f.ret + 1]).replace('ARGS', ', '.join(args))
		if (f.type == 'void'):
			body = statement
			ret = ';'
		else:
			body = ''
			ret = ' ' + statement
		functs.append('LF_WEAK ' + str(f) + ' {\nBODY\treturnVALUE\n}\n'.replace('BODY', body).replace('VALUE', ret))
	ctemplate = ctemplate.replace('VARIABLES\n\n', '')
	ctemplate = ctemplate.replace('STRUCT', '\t' + ',\n\t'.join(struct))
	ctemplate = ctemplate.replace('FUNCTIONS', '\n'.join(functs))

	outc.write(ctemplate)


if __name__ == '__main__':
	if len(sys.argv) > 2:
		process_file(sys.argv[1], sys.argv[2])
	else:
		print ('fdwarf package.elf NAME')
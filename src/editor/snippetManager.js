
import { state } from '../state.js';

/**
 * Register snippet manager for an editor instance
 * @param {monaco.editor.IStandaloneCodeEditor} editor 
 * @param {any} monaco 
 */
export function registerSnippetManager(editor, monaco) {
  // Feature 2: Auto-item Insertion
  editor.onKeyDown((e) => {
    if (e.keyCode === monaco.KeyCode.Enter) {
      handleEnterKey(editor, monaco, e);
    }
  });

  // Feature 3 & 4: Math Delimiters & Brackets
  editor.onDidType((text) => {
    handleTyping(editor, monaco, text);
  });
}

/**
 * Handle Enter key for auto-item insertion
 */
function handleEnterKey(editor, monaco, event) {
  const model = editor.getModel();
  const position = editor.getPosition();
  const lineContent = model.getLineContent(position.lineNumber);
  
  // Check if we are in a list environment
  if (isInListEnvironment(model, position.lineNumber)) {
    // Check if current line is empty item
    if (lineContent.trim() === '\\item') {
      event.preventDefault();
      event.stopPropagation();
      
      // Remove the empty item and unindent (or just end line)
      editor.executeEdits('auto-item', [{
        range: new monaco.Range(position.lineNumber, 1, position.lineNumber, lineContent.length + 1),
        text: '',
        forceMoveMarkers: true
      }]);
      
      // Move to next line (which is now empty or next content)
      // Actually, if we remove the line, we are done.
      // But we might want to exit the list? 
      // Standard behavior: remove \item and just have a newline.
      // Since we prevented default, we need to insert the newline manually if we want one.
      // But if we are "exiting", maybe we just want to remove the \item.
      // Let's just remove the \item.
      return;
    }

    // If normal enter, let it happen, then insert \item?
    // No, we want to prevent default and insert newline + \item
    // But only if the cursor is at the end of the line?
    // Or anywhere?
    
    // Let's only trigger if we are not inside a command or something weird.
    // Simple heuristic: if line starts with \item, next line should too.
    
    if (lineContent.trim().startsWith('\\item')) {
      // We can't easily prevent default and do async stuff or complex edits without breaking undo stack sometimes.
      // But here we can just execute edits.
      
      // We need to wait for the newline to happen? No, we prevent default.
      event.preventDefault();
      event.stopPropagation();
      
      const indent = lineContent.match(/^\s*/)[0];
      editor.executeEdits('auto-item', [{
        range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column),
        text: '\n' + indent + '\\item ',
        forceMoveMarkers: true
      }]);
      
      // Scroll to cursor
      editor.revealPosition(editor.getPosition());
    }
  }
}

/**
 * Check if line is inside itemize or enumerate
 * This is a simple heuristic looking backwards
 */
function isInListEnvironment(model, lineNumber) {
  let depth = 0;
  for (let i = lineNumber; i >= 1; i--) {
    const line = model.getLineContent(i);
    if (line.includes('\\end{itemize}') || line.includes('\\end{enumerate}')) {
      depth--;
    }
    if (line.includes('\\begin{itemize}') || line.includes('\\begin{enumerate}')) {
      depth++;
    }
    if (depth > 0) return true;
  }
  return false;
}

const unicodeMathMap = {
  '→': '\\rightarrow',
  '⇒': '\\Rightarrow',
  'α': '\\alpha',
  'β': '\\beta',
  'γ': '\\gamma',
  'δ': '\\delta',
  'ε': '\\epsilon',
  'θ': '\\theta',
  'λ': '\\lambda',
  'μ': '\\mu',
  'π': '\\pi',
  'σ': '\\sigma',
  'τ': '\\tau',
  'φ': '\\phi',
  'ω': '\\omega',
  '∞': '\\infty',
  '∑': '\\sum',
  '∏': '\\prod',
  '∫': '\\int',
  '∂': '\\partial',
  '∇': '\\nabla',
  '√': '\\sqrt',
  '≈': '\\approx',
  '≠': '\\neq',
  '≤': '\\leq',
  '≥': '\\geq',
  '∈': '\\in',
  '⊂': '\\subset',
  '∪': '\\cup',
  '∩': '\\cap'
};

/**
 * Handle typing for math delimiters
 */
function handleTyping(editor, monaco, text) {
  if (text === '(' || text === '[' || text === '{') {
    handleOpeningBracket(editor, monaco, text);
  } else if (unicodeMathMap[text]) {
    handleUnicodeMath(editor, monaco, text);
  } else {
    handleSubSuperscript(editor, monaco, text);
  }
}

function handleUnicodeMath(editor, monaco, char) {
  const model = editor.getModel();
  const position = editor.getPosition();
  
  if (!isInMathMode(model, position)) return;
  
  const latexCmd = unicodeMathMap[char];
  
  // Replace the unicode char with latex command
  editor.executeEdits('unicode-math', [{
    range: new monaco.Range(position.lineNumber, position.column - 1, position.lineNumber, position.column),
    text: latexCmd + ' ', // Add space for convenience
    forceMoveMarkers: true
  }]);
}

function handleSubSuperscript(editor, monaco, char) {
  // Only handle alphanumeric chars
  if (!/[a-zA-Z0-9]/.test(char)) return;

  const model = editor.getModel();
  const position = editor.getPosition();
  
  if (!isInMathMode(model, position)) return;
  
  const lineContent = model.getLineContent(position.lineNumber);
  // Check if we have something like x^2 where 2 is the char we just typed
  // textBefore includes the char we just typed? No, position is after the char.
  // So textBefore is ...x^2
  const textBefore = lineContent.substring(0, position.column);
  
  // Regex to match: (anything)(_ or ^)(single char)(current char)
  // We want to match if the previous char was a single char argument to _ or ^
  // e.g. x^2 -> x^{23} when 3 is typed
  
  // Look at the last 3 chars: ^23 (where 3 is current char)
  if (textBefore.length < 3) return;
  
  const last3 = textBefore.slice(-3); // e.g. "^23"
  const trigger = last3[0];
  const firstArg = last3[1];
  const secondArg = last3[2]; // current char
  
  if ((trigger === '^' || trigger === '_') && 
      /[a-zA-Z0-9]/.test(firstArg) && 
      secondArg === char) {
        
    // We found a pattern like ^23. Convert to ^{23}
    editor.executeEdits('smart-braces', [{
      range: new monaco.Range(position.lineNumber, position.column - 2, position.lineNumber, position.column),
      text: `{${firstArg}${secondArg}}`,
      forceMoveMarkers: true
    }]);
  }
}

function handleOpeningBracket(editor, monaco, char) {
  const model = editor.getModel();
  const position = editor.getPosition();
  
  // Check if we are in math mode
  if (!isInMathMode(model, position)) return;
  
  const lineContent = model.getLineContent(position.lineNumber);
  const textBefore = lineContent.substring(0, position.column - 1); // -1 because char is already typed? 
  // Wait, onDidType fires AFTER the text is inserted?
  // "An event emitted when the text has been typed in the editor."
  // Yes. So the char is already in the model.
  
  // Check for \left prefix
  if (textBefore.endsWith('\\left' + char)) {
    // Insert matching \right
    const closing = getMatchingDelimiter(char);
    const insertText = ` \\right${closing}`;
    
    editor.executeEdits('math-delimiter', [{
      range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column),
      text: insertText,
      forceMoveMarkers: true
    }]);
    
    // Move cursor back between delimiters
    editor.setPosition({
      lineNumber: position.lineNumber,
      column: position.column + 1 // space
    });
  } else {
    // Just auto-close bracket in math mode
    const closing = getMatchingBracket(char);
    
    editor.executeEdits('math-bracket', [{
      range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column),
      text: closing,
      forceMoveMarkers: true
    }]);
    
    // Move cursor back
    editor.setPosition({
      lineNumber: position.lineNumber,
      column: position.column
    });
  }
}

function getMatchingDelimiter(char) {
  switch (char) {
    case '(': return ')';
    case '[': return ']';
    case '{': return '\\}'; // \left{ needs \right\}
    default: return '';
  }
}

function getMatchingBracket(char) {
  switch (char) {
    case '(': return ')';
    case '[': return ']';
    case '{': return '}';
    default: return '';
  }
}

/**
 * Check if position is in math mode
 * Heuristic: count $ signs on the line?
 * Or check for environment?
 */
function isInMathMode(model, position) {
  const lineContent = model.getLineContent(position.lineNumber);
  const textBefore = lineContent.substring(0, position.column - 1);
  
  // Simple check for inline math $...$
  const dollarCount = (textBefore.match(/\$/g) || []).length;
  if (dollarCount % 2 === 1) return true;
  
  // Check for \[ ...
  if (textBefore.lastIndexOf('\\[') > textBefore.lastIndexOf('\\]')) return true;
  
  // Check for environments (equation, align, etc)
  // This requires scanning backwards
  return isInMathEnvironment(model, position.lineNumber);
}

function isInMathEnvironment(model, lineNumber) {
  let depth = 0;
  const mathEnvs = ['equation', 'align', 'gather', 'multline'];
  
  for (let i = lineNumber; i >= 1; i--) {
    const line = model.getLineContent(i);
    for (const env of mathEnvs) {
      if (line.includes(`\\end{${env}}`)) depth--;
      if (line.includes(`\\begin{${env}}`)) depth++;
    }
    if (depth > 0) return true;
  }
  return false;
}
